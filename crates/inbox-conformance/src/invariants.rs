//! `expect: value_invariants`, read and observed as ESS 0.96.0 observes it
//! (`value_invariant.rs`, `Contract::observe`): every row of the view is checked, its value at the
//! root against the declared representation, and every occurrence of the value type at the site
//! against the type's invariants; and a row the witness selects must hold one occurrence.

use std::collections::BTreeMap;

use inbox::server::json::Value;

use crate::suite::Op;
use crate::value::{exact_equal, render};

/// A canonical type of a declaration, as far as this runner reads one.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    /// `Optional<T>`.
    Optional(Box<TypeRef>),
    /// `List<T>`.
    List(Box<TypeRef>),
    /// A primitive.
    Primitive(Primitive),
    /// A declaration of the expectation, by qualified name.
    Named(String),
}

/// The primitives whose JSON carriage this runner checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    /// `String`, `Timestamp`, `Duration`, `Uuid`, `Bytes`: carried as text.
    Text,
    /// `Boolean`.
    Boolean,
    /// `Integer`: a number an `i64` holds.
    Integer,
    /// `Decimal`: a number, or text that reads as one.
    Decimal,
    /// `Json`: anything.
    Json,
    /// `Binary64`: outside the observer's profile; ESS refuses an expectation reaching one.
    Binary64,
}

/// One declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// A newtype: its representation and its text constraints.
    Newtype {
        /// The representation.
        of: TypeRef,
        /// The characters every value is drawn from.
        alphabet: Option<String>,
        /// The text every value starts with.
        prefix: Option<String>,
    },
    /// A struct: its fields in declaration order.
    Struct {
        /// The fields.
        fields: Vec<(String, TypeRef)>,
    },
    /// An enum: its labels.
    Enum {
        /// The labels.
        variants: Vec<String>,
    },
}

/// One step of the site inside the root value.
#[derive(Debug, Clone, PartialEq)]
pub enum PathOp {
    /// `present`: inside an `Optional`.
    Present,
    /// `field`: a struct member.
    Field(String),
    /// `element`: an element of a list.
    Element,
}

/// `expect: value_invariants`, admitted.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueInvariants {
    /// The row member the site is under.
    pub root: String,
    /// Its declared type.
    pub root_type: TypeRef,
    /// Every declaration the root reaches.
    pub types: BTreeMap<String, Body>,
    /// The newtype whose invariants are checked.
    pub value_type: String,
    /// Its invariants, each `value.count <op> <n>`, with the statement for a message.
    pub counts: Vec<(String, Op, f64)>,
    /// The site.
    pub path: Vec<PathOp>,
    /// The row member that holds the identity of a row the witness selects.
    pub identity: String,
    /// The event whose first occurrence in the scenario carries that identity.
    pub event: String,
    /// The payload member of that event.
    pub field: String,
    /// Why ESS's `Contract::admit` refuses the expectation, if it does: a declaration the root
    /// or the witness's identity reaches that is not declared, one declared and not reached, a
    /// `Binary64`, a value type with no invariants. The step is then an `error`, as ESS records a
    /// broken verdict, and nothing is observed.
    pub broken: Option<String>,
}

/// Parses a canonical type spelling. A name that is no primitive is a declaration's; whether it
/// is declared is [`closure`]'s question.
///
/// # Errors
///
/// A spelling this runner does not read: a `Map` or any other generic but `Optional` and `List`.
pub fn parse_type(raw: &str) -> Result<TypeRef, String> {
    if let Some(inner) = raw
        .strip_prefix("Optional<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return Ok(TypeRef::Optional(Box::new(parse_type(inner)?)));
    }
    if let Some(inner) = raw
        .strip_prefix("List<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return Ok(TypeRef::List(Box::new(parse_type(inner)?)));
    }
    let primitive = match raw {
        "String" | "Timestamp" | "Duration" | "Uuid" | "Bytes" => Primitive::Text,
        "Boolean" => Primitive::Boolean,
        "Integer" => Primitive::Integer,
        "Decimal" => Primitive::Decimal,
        "Json" => Primitive::Json,
        "Binary64" => Primitive::Binary64,
        name if !name.is_empty() && !name.contains(['<', '>', ',', ' ']) => {
            return Ok(TypeRef::Named(name.to_owned()));
        }
        other => {
            return Err(format!(
                "the type `{other}` is not one this runner checks a value of"
            ));
        }
    };
    Ok(TypeRef::Primitive(primitive))
}

/// ESS `Contract::closure`: every declaration `starts` reach is declared, every declaration is
/// reached, and no `Binary64` is reached.
///
/// # Errors
///
/// The first of those that does not hold, in ESS's words.
pub fn closure(starts: Vec<TypeRef>, types: &BTreeMap<String, Body>) -> Result<(), String> {
    let mut reached = std::collections::BTreeSet::new();
    let mut pending = starts;
    while let Some(type_ref) = pending.pop() {
        match type_ref {
            TypeRef::Primitive(Primitive::Binary64) => {
                return Err("a Binary64 value is outside this observer's profile".to_owned());
            }
            TypeRef::Primitive(_) => {}
            TypeRef::Optional(of) | TypeRef::List(of) => pending.push(*of),
            TypeRef::Named(name) => {
                if !reached.insert(name.clone()) {
                    continue;
                }
                match types.get(&name) {
                    None => return Err(format!("`{name}` is reached and not declared")),
                    Some(Body::Newtype { of, .. }) => pending.push(of.clone()),
                    Some(Body::Struct { fields }) => {
                        pending.extend(fields.iter().map(|(_, type_ref)| type_ref.clone()));
                    }
                    Some(Body::Enum { .. }) => {}
                }
            }
        }
    }
    if let Some(unreached) = types.keys().find(|name| !reached.contains(*name)) {
        return Err(format!("`{unreached}` is declared and not reached"));
    }
    Ok(())
}

impl ValueInvariants {
    /// Observes one read: `Ok` where it holds, `Err` naming what does not.
    ///
    /// `witness` is the identity the witness row carries. Every row is checked; a row whose root
    /// is malformed adds a failure and supplies no occurrence. A row with `identity` equal to
    /// `witness` that holds an occurrence is required.
    pub fn observe(&self, rows: &[Value], witness: &Value) -> Result<(), Observed> {
        if let Some(reason) = &self.broken {
            return Err(Observed::Broken(reason.clone()));
        }
        let mut failures = Vec::new();
        let mut occurred = false;
        for (index, row) in rows.iter().enumerate() {
            let value = row.member(&self.root);
            let location = format!("row {index} `{}`", self.root);
            if let Err(reason) = self.shape(&self.root_type, value, &location) {
                failures.push(reason);
                continue;
            }
            let mut terminals = Vec::new();
            self.visit(
                &self.root_type,
                value,
                &mut Vec::new(),
                &location,
                &mut terminals,
            );
            let eligible = row
                .member(&self.identity)
                .is_some_and(|carried| exact_equal(carried, witness));
            for (location, terminal) in terminals {
                let Value::Text(text) = terminal else {
                    return Err(Observed::Broken(format!(
                        "the invariants of `{}` cannot be decided of {} at {location}",
                        self.value_type,
                        render(terminal)
                    )));
                };
                let scalars = text.chars().count() as f64;
                for (statement, op, bound) in &self.counts {
                    if !op.holds(scalars, *bound) {
                        failures.push(format!(
                            "{location} holds {}, and `{statement}` does not hold of it",
                            render(terminal)
                        ));
                    }
                }
                if eligible {
                    occurred = true;
                }
            }
        }
        if !failures.is_empty() {
            return Err(Observed::Unsatisfied(failures.join("; ")));
        }
        if !occurred {
            return Err(Observed::Unsatisfied(format!(
                "no selected row holds an actual occurrence of `{}` at `{}`; legal absence and empty collections witness none",
                self.value_type, self.root
            )));
        }
        Ok(())
    }

    /// The whole value against its declared representation.
    fn shape(
        &self,
        type_ref: &TypeRef,
        value: Option<&Value>,
        location: &str,
    ) -> Result<(), String> {
        if let TypeRef::Optional(of) = type_ref {
            return match value {
                None | Some(Value::Null) => Ok(()),
                Some(present) => self.shape(of, Some(present), location),
            };
        }
        let Some(value) = value else {
            return if self.represents_absence(type_ref) {
                Ok(())
            } else {
                Err(format!("{location} is missing, and its type is required"))
            };
        };
        match type_ref {
            TypeRef::Optional(_) => unreachable!("handled above"),
            TypeRef::Primitive(primitive) => {
                if admits(*primitive, value) {
                    Ok(())
                } else {
                    Err(format!(
                        "{location} holds {}, which is no {primitive:?}",
                        render(value)
                    ))
                }
            }
            TypeRef::List(of) => {
                let Value::Array(items) = value else {
                    return Err(format!(
                        "{location} holds {}, which is no list",
                        render(value)
                    ));
                };
                for (index, item) in items.iter().enumerate() {
                    self.shape(of, Some(item), &format!("{location}[{index}]"))?;
                }
                Ok(())
            }
            TypeRef::Named(name) => match &self.types[name] {
                Body::Newtype {
                    of,
                    alphabet,
                    prefix,
                } => {
                    self.shape(of, Some(value), location)?;
                    if let Value::Text(text) = value {
                        if let Some(alphabet) = alphabet
                            && let Some(outside) = text.chars().find(|c| !alphabet.contains(*c))
                        {
                            return Err(format!(
                                "{location} holds {text:?}, whose {outside:?} is outside the alphabet of `{name}`"
                            ));
                        }
                        if let Some(prefix) = prefix
                            && !text.starts_with(prefix.as_str())
                        {
                            return Err(format!(
                                "{location} holds {text:?}, which does not start with the prefix {prefix:?} of `{name}`"
                            ));
                        }
                    }
                    Ok(())
                }
                Body::Struct { fields } => {
                    let Value::Object(members) = value else {
                        return Err(format!(
                            "{location} holds {}, which is no `{name}`",
                            render(value)
                        ));
                    };
                    if let Some((undeclared, _)) = members
                        .iter()
                        .find(|(key, _)| !fields.iter().any(|(field, _)| field == key))
                    {
                        return Err(format!(
                            "{location} carries `{undeclared}`, which `{name}` does not declare"
                        ));
                    }
                    for (field, type_ref) in fields {
                        self.shape(
                            type_ref,
                            value.member(field),
                            &format!("{location}.{field}"),
                        )?;
                    }
                    Ok(())
                }
                Body::Enum { variants } => match value {
                    Value::Text(label) if variants.contains(label) => Ok(()),
                    other => Err(format!(
                        "{location} holds {}, which is no label of `{name}`",
                        render(other)
                    )),
                },
            },
        }
    }

    fn represents_absence(&self, type_ref: &TypeRef) -> bool {
        match type_ref {
            TypeRef::Optional(_) => true,
            TypeRef::Named(name) => match &self.types[name] {
                Body::Newtype { of, .. } => self.represents_absence(of),
                _ => false,
            },
            _ => false,
        }
    }

    /// Every value of the checked type at the site, with where it was met.
    fn visit<'v>(
        &self,
        type_ref: &TypeRef,
        value: Option<&'v Value>,
        path: &mut Vec<PathOp>,
        location: &str,
        terminals: &mut Vec<(String, &'v Value)>,
    ) {
        let Some(value) = value else {
            return;
        };
        if let TypeRef::Named(name) = type_ref
            && *name == self.value_type
            && *path == self.path
        {
            terminals.push((location.to_owned(), value));
        }
        match type_ref {
            TypeRef::Primitive(_) => {}
            TypeRef::Optional(of) => {
                if *value != Value::Null {
                    path.push(PathOp::Present);
                    self.visit(of, Some(value), path, location, terminals);
                    path.pop();
                }
            }
            TypeRef::List(of) => {
                if let Value::Array(items) = value {
                    for (index, item) in items.iter().enumerate() {
                        path.push(PathOp::Element);
                        self.visit(
                            of,
                            Some(item),
                            path,
                            &format!("{location}[{index}]"),
                            terminals,
                        );
                        path.pop();
                    }
                }
            }
            TypeRef::Named(name) => match &self.types[name] {
                Body::Newtype { .. } | Body::Enum { .. } => {}
                Body::Struct { fields } => {
                    if let Value::Object(_) = value {
                        for (field, type_ref) in fields {
                            let Some(member) = value.member(field) else {
                                continue;
                            };
                            path.push(PathOp::Field(field.clone()));
                            self.visit(
                                type_ref,
                                Some(member),
                                path,
                                &format!("{location}.{field}"),
                                terminals,
                            );
                            path.pop();
                        }
                    }
                }
            },
        }
    }
}

/// Why an observation does not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// The rows break what the type declares, or hold no occurrence where one is required.
    Unsatisfied(String),
    /// An invariant cannot be decided of a value.
    Broken(String),
}

fn admits(primitive: Primitive, value: &Value) -> bool {
    match primitive {
        Primitive::Text => matches!(value, Value::Text(_)),
        Primitive::Boolean => matches!(value, Value::Bool(_)),
        Primitive::Integer => {
            matches!(value, Value::Number(spelling) if spelling.parse::<i64>().is_ok())
        }
        Primitive::Decimal => match value {
            Value::Number(_) => true,
            Value::Text(text) => text.parse::<f64>().is_ok(),
            _ => false,
        },
        Primitive::Json => true,
        Primitive::Binary64 => false,
    }
}
