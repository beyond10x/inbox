//! The cases of `inbox-core`, in one test binary: each file of this directory and of `core/` is a
//! module.

mod adversary;
mod adversary2;
mod adversary_deadline;
mod adversary_deadline_2;
mod adversary_w6_from;
mod adversary_w7_id;
mod adversary_w7_key;
mod adversary_w8_expire;
mod adversary_w8_fields;
mod adversary_w8_order;
mod adversary_w8_read;
#[path = "core/backend.rs"]
mod backend;
#[path = "core/branches.rs"]
mod branches;
mod deadline;
mod dedup_key_scope;
mod expire_once;
mod foreign_message;
#[path = "core/obligations.rs"]
mod obligations;
mod post_message_id;
#[path = "core/post_read_answer.rs"]
mod post_read_answer;
#[path = "core/precedence.rs"]
mod precedence;
#[path = "core/section.rs"]
mod section;
#[path = "core/support.rs"]
mod support;
