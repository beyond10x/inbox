"""Writes the data of the landing page's animation and pitch graphics from the compiled specification.

usage: animation.py <ess directory> [--ir <file>] [--check]

It compiles <ess directory> with `ess specify compile --format json` (or reads the compiled IR from
--ir <file>), builds the story of one message through the inbox from that IR, checks it, and only
then writes website/src/generated/animation.json. A failed check changes no file. With --check it
writes nothing and exits 1 when the file differs from what it would write, printing one line per
path: `edited:` or `missing:`.

Every name the data carries (commands, views, errors, states, actors, the entities) is read from
the IR. This file holds only layout, timing, the vocabulary of the IR format and the rules that
select from the IR:

- the message entity is the one entity whose lifecycle declares terminal states; the inbox entity
  is the entity it references;
- the creating actor is the actor that may send the command that creates a message;
- the first journey walks the longest path of the creating actor's transitions from the initial
  state to the first terminal state, each further journey the shortest path to the next terminal
  state; each journey ends with the creating actor's command that deletes a message in that state;
- each command on the message entity that another actor sends gets a journey of its own, with its
  default (`otherwise`) outcome;
- the command that creates the inbox opens the story, the command of the transition that leaves the
  inbox's initial state closes it;
- a command is the recipient's when it acts on the inbox entity, or when one of its refusals decided
  by the caller is `<reference field> != caller.<attribute>`; otherwise it is the sender's;
- the views of a step are those its actor may read, over the step's entity, whose filter admits the
  state after the step (before it, for a step that deletes);
- the refusals of a step are the errors its command declares; every error of the IR is shown once,
  at a step whose command declares it, scarcest first.

The story fails loudly when a command, a view or an error of the IR has no place in it, or when a
frame shows more than WORD_LIMIT words.
"""

import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join("website", "src", "generated", "animation.json")
WORD_LIMIT = 12
FRAME_MS = 2600
LAYOUT = {"host": "host", "process": "process", "service": "inbox", "planned": "planned"}
LAYOUT_TEXTS = [LAYOUT["host"], LAYOUT["host"], LAYOUT["process"], LAYOUT["service"], LAYOUT["planned"]]
PLANNED_NOTE = ("The second host and the link to it are planned: today the service listens on loopback "
                "only, so every agent that uses it runs on the service's own host.")
GENERATED = "Written by tools/animation.py from the compiled ess/ (task animation). Do not edit."


class StoryError(Exception):
    pass


def short(name):
    return name.rsplit(".", 1)[-1]


def plain(text):
    return " ".join((text or "").replace("`", "").split())


def compile_ir(ess):
    run = subprocess.run(["ess", "specify", "compile", "--path", ess, "--format", "json"],
                         capture_output=True, text=True, check=False)
    if run.returncode != 0:
        raise StoryError(f"ess specify compile failed:\n{run.stderr.strip()}")
    return json.loads(run.stdout)


def one(items, what):
    if len(items) != 1:
        raise StoryError(f"expected exactly one {what}, found {len(items)}: {sorted(map(str, items))}")
    return items[0]


def successes(command):
    return [o for o in command["outcomes"] if "error" not in o]


def subject_entity(command):
    entities = {o["subject"]["entity"] for o in successes(command) if o.get("subject")}
    return one(sorted(entities), f"subject entity of {command['name']}")


def refusals(command):
    seen = []
    for outcome in command["outcomes"]:
        name = outcome.get("error")
        if name and short(name) not in seen:
            seen.append(short(name))
    return seen


def admits(predicate, state):
    if predicate is None:
        return True
    if isinstance(predicate, str):
        match = re.fullmatch(r"state == (\w+)", predicate.strip())
        return match.group(1) == state if match else True
    if "all" in predicate:
        return all(admits(p, state) for p in predicate["all"])
    if "any" in predicate:
        return any(admits(p, state) for p in predicate["any"])
    return True


class Spec:
    def __init__(self, ir):
        self.ir = ir
        self.commands = ir["commands"]
        self.views = ir["views"]
        self.errors = ir["errors"]
        self.actors = ir["actors"]
        message = one([n for n, e in sorted(ir["entities"].items())
                       if (e.get("lifecycle") or {}).get("terminal")], "entity with terminal states")
        self.message = message
        self.lifecycle = ir["entities"][message]["lifecycle"]
        relation = one([r for r in ir["entities"][message].get("relations") or []
                        if r.get("kind") == "references"
                        and (ir["entities"].get(r["target"], {}).get("lifecycle"))],
                       f"referenced entity with a lifecycle of {message}")
        self.inbox = relation["target"]
        self.via = relation["via"]
        self.inbox_lifecycle = ir["entities"][self.inbox]["lifecycle"]
        self.actor_of = {}
        for name in sorted(self.commands):
            self.actor_of[name] = one([a for a, actor in sorted(self.actors.items())
                                       if name in actor.get("may", [])], f"actor that may send {name}")
        self.create_message = self.creating(message)
        self.create_inbox = self.creating(self.inbox)
        self.creator = self.actor_of[self.create_message]

    def creating(self, entity):
        return one([n for n, c in sorted(self.commands.items())
                    if any((o.get("subject") or {}).get("effect") == "creates"
                           and o["subject"]["entity"] == entity for o in successes(c))],
                   f"command that creates {entity}")

    def transition_command(self, entity, transition):
        found = []
        for name, command in sorted(self.commands.items()):
            for outcome in successes(command):
                subject = outcome.get("subject") or {}
                if subject.get("entity") == entity and (subject.get("transition") or {}).get("name") == transition:
                    found.append((name, outcome["name"]))
        return one(found, f"command of the transition {transition} of {entity}")

    def creator_graph(self):
        edges = []
        for transition in self.lifecycle["transitions"]:
            command, outcome = self.transition_command(self.message, transition["name"])
            if self.actor_of[command] == self.creator:
                for source in transition["from"]:
                    edges.append((source, transition["to"], command, outcome))
        return edges

    def paths(self, target):
        edges = self.creator_graph()
        found = []

        def walk(state, path, visited):
            if state == target:
                found.append(list(path))
                return
            for source, to, command, outcome in edges:
                if source == state and to not in visited:
                    path.append((to, command, outcome))
                    walk(to, path, visited | {to})
                    path.pop()

        walk(self.lifecycle["initial"], [], {self.lifecycle["initial"]})
        if not found:
            raise StoryError(f"no path of {short(self.creator)} from {self.lifecycle['initial']} to {target}")
        return found

    def deleting(self, state):
        found = []
        for name, command in sorted(self.commands.items()):
            if self.actor_of[name] != self.creator:
                continue
            for outcome in successes(command):
                subject = outcome.get("subject") or {}
                condition = outcome["condition"]
                states = condition.get("state")
                states = [states] if isinstance(states, str) else states or []
                if (subject.get("effect") == "deletes" and subject.get("entity") == self.message
                        and condition.get("kind") == "subject_state" and state in states):
                    found.append((name, outcome["name"]))
        return one(found, f"command of {short(self.creator)} that deletes a message in {state}")

    def role(self, command):
        if self.actor_of[command] != self.creator:
            return "service"
        if subject_entity(self.commands[command]) == self.inbox:
            return "recipient"
        attributes = [a["name"] for a in self.actors[self.creator].get("attributes", [])]
        owned = {f"{self.via} != caller.{a}" for a in attributes}
        for outcome in self.commands[command]["outcomes"]:
            predicate = outcome["condition"].get("predicate")
            if outcome.get("decided_by_caller") and isinstance(predicate, str) and predicate.strip() in owned:
                return "recipient"
        return "sender"

    def views_for(self, command, entity, state):
        readable = set(self.actors[self.actor_of[command]].get("may_read", []))
        return [short(n) for n, v in sorted(self.views.items())
                if n in readable and v["source"] == entity and admits(v.get("filter"), state)]


def build(ir):
    spec = Spec(ir)
    steps = []

    def step(command, outcome, journey, state_before):
        entity = subject_entity(spec.commands[command])
        result = next(o for o in spec.commands[command]["outcomes"] if o["name"] == outcome)
        subject = result.get("subject") or {}
        effect = subject.get("effect")
        role = spec.role(command)
        if effect == "creates":
            state = (spec.lifecycle if entity == spec.message else spec.inbox_lifecycle)["initial"]
        elif effect == "moves":
            state = subject["transition"]["to"]
        else:
            state = state_before
        removed = effect == "deletes"
        if entity == spec.inbox:
            token = None
        elif removed:
            token = "sender" if role == "sender" else "inbox"
        elif effect == "creates" or state in spec.lifecycle["terminal"]:
            token = "inbox"
        else:
            token = role
        steps.append({
            "actor": role,
            "command": command,
            "entity": entity,
            "journey": journey,
            "outcome": outcome,
            "removed": removed,
            "state": state,
            "token": token,
            "views": spec.views_for(command, entity, state),
        })
        return state

    step(spec.create_inbox, one([o["name"] for o in successes(spec.commands[spec.create_inbox])
                                 if (o.get("subject") or {}).get("effect") == "creates"],
                                "creating outcome"), 0, None)
    create_outcome = one([o["name"] for o in successes(spec.commands[spec.create_message])
                          if (o.get("subject") or {}).get("effect") == "creates"], "creating outcome")
    journey = 0
    for index, terminal in enumerate(spec.lifecycle["terminal"]):
        journey += 1
        paths = spec.paths(terminal)
        path = max(paths, key=len) if index == 0 else min(paths, key=len)
        state = step(spec.create_message, create_outcome, journey, None)
        for _to, command, outcome in path:
            state = step(command, outcome, journey, state)
        command, outcome = spec.deleting(terminal)
        step(command, outcome, journey, state)
    for name in sorted(spec.commands):
        if spec.actor_of[name] != spec.creator and subject_entity(spec.commands[name]) == spec.message:
            journey += 1
            state = step(spec.create_message, create_outcome, journey, None)
            outcome = one([o["name"] for o in successes(spec.commands[name])
                           if o["condition"]["kind"] == "otherwise"], f"default outcome of {name}")
            step(name, outcome, journey, state)
    initial = spec.inbox_lifecycle["initial"]
    leaving = one([t["name"] for t in spec.inbox_lifecycle["transitions"] if initial in t["from"]],
                  f"transition leaving {initial}")
    command, outcome = spec.transition_command(spec.inbox, leaving)
    step(command, outcome, 0, initial)

    missing = sorted(set(spec.commands) - {s["command"] for s in steps})
    if missing:
        raise StoryError(f"commands with no place in the story: {', '.join(map(short, missing))}")
    shown_views = {v for s in steps for v in s["views"]}
    missing = sorted(set(map(short, spec.views)) - shown_views)
    if missing:
        raise StoryError(f"views with no place in the story: {', '.join(missing)}")

    offered = [refusals(spec.commands[s["command"]]) for s in steps]
    assigned = [[] for _ in steps]
    errors = sorted(map(short, spec.errors))
    scarcity = {e: sum(e in o for o in offered) for e in errors}
    missing = [e for e in errors if not scarcity[e]]
    if missing:
        raise StoryError(f"errors no command of the story declares: {', '.join(missing)}")
    for error in sorted(errors, key=lambda e: (scarcity[e], e)):
        candidates = [i for i, o in enumerate(offered) if error in o]
        target = min(candidates, key=lambda i: (len(assigned[i]), i))
        assigned[target].append(error)
    shown = {e: 1 for e in errors}
    for i, chosen in enumerate(assigned):
        if not chosen and offered[i]:
            pick = min(offered[i], key=lambda e: (shown[e], offered[i].index(e)))
            shown[pick] += 1
            chosen.append(pick)
        chosen.sort(key=offered[i].index)

    names = {
        "sender": short(spec.creator),
        "recipient": short(spec.creator),
        "service": one(sorted({short(spec.actor_of[s["command"]]) for s in steps if s["actor"] == "service"}),
                       "actor of the service's own commands"),
    }
    inbox_name = short(spec.inbox)
    frames = []
    for i, s in enumerate(steps):
        command = spec.commands[s["command"]]
        lifecycle = spec.lifecycle if s["entity"] == spec.message else spec.inbox_lifecycle
        state_text = None if s["removed"] else s["state"]
        for refusal in assigned[i] or [None]:
            visible = LAYOUT_TEXTS + [short(s["command"])]
            if state_text:
                visible.append(state_text)
            visible += s["views"]
            if refusal:
                visible.append(refusal)
            words = sum(len(text.split()) for text in visible)
            if words > WORD_LIMIT:
                raise StoryError(f"frame {len(frames)} ({short(s['command'])}) shows {words} words, "
                                 f"more than {WORD_LIMIT}: {visible}")
            summary = plain(command.get("naming", {}).get("summary"))
            state_summary = plain(((lifecycle.get("naming") or {}).get(s["state"]) or {}).get("summary"))
            parts = [f"{names[s['actor']]}, the {s['actor']}, sends {short(s['command'])}. {summary}"]
            if s["removed"]:
                parts.append(f"The {short(s['entity']).lower()} is removed.")
            else:
                parts.append(f"State: {s['state']}." + (f" {state_summary}" if state_summary else ""))
            if s["views"]:
                parts.append(("Found in " if s["removed"] else "Visible in ") + ", ".join(s["views"]) + ".")
            if refusal:
                error = spec.errors[next(n for n in spec.errors if short(n) == refusal)]
                parts.append(f"{short(s['command'])} can be refused with {refusal}: {plain(error.get('summary'))}")
            frames.append({
                "actor": s["actor"],
                "command": short(s["command"]),
                "description": " ".join(p for p in parts if p),
                "end": spec.lifecycle["terminal"].index(s["state"])
                if s["entity"] == spec.message and s["state"] in spec.lifecycle["terminal"] else None,
                "entity": short(s["entity"]),
                "journey": s["journey"],
                "refusal": refusal,
                "removed": s["removed"],
                "state": state_text,
                "step": i,
                "token": s["token"],
                "views": s["views"],
                "visible": visible,
                "words": words,
            })

    order = []
    for kind in ("command", "views", "refusal"):
        seen = []
        for frame in frames:
            values = frame[kind] if isinstance(frame[kind], list) else [frame[kind]]
            for value in values:
                if value and value not in seen:
                    seen.append(value)
        order.append(seen)
    first = steps_of_journey(steps, 1)
    return {
        "actors": names,
        "commands": order[0],
        "counts": {"commands": len(spec.commands), "errors": len(spec.errors), "views": len(spec.views)},
        "errors": order[2],
        "frame_ms": FRAME_MS,
        "frames": frames,
        "generated": GENERATED,
        "inbox": inbox_name,
        "message": short(spec.message),
        "still": [{"actor": s["actor"], "command": short(s["command"]),
                   "toward": "inbox" if s["token"] == "inbox" else "actor"} for s in first],
        "layout": LAYOUT,
        "lifecycle": {
            "initial": spec.lifecycle["initial"],
            "path": [s["state"] for s in first if not s["removed"] and s["state"] not in spec.lifecycle["terminal"]],
            "terminal": list(spec.lifecycle["terminal"]),
        },
        "planned_note": PLANNED_NOTE,
        "views": order[1],
        "word_limit": WORD_LIMIT,
    }


def steps_of_journey(steps, journey):
    return [s for s in steps if s["journey"] == journey]


def render(data):
    return json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv):
    args = list(argv)
    check = "--check" in args
    if check:
        args.remove("--check")
    ir_file = None
    if "--ir" in args:
        at = args.index("--ir")
        ir_file = args[at + 1]
        del args[at:at + 2]
    if len(args) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        if ir_file:
            with open(ir_file, encoding="utf-8") as handle:
                ir = json.load(handle)
        else:
            ir = compile_ir(args[0])
        text = render(build(ir))
    except StoryError as error:
        print(f"animation.py: {error}", file=sys.stderr)
        return 1
    path = os.path.join(ROOT, OUT)
    if check:
        if not os.path.exists(path):
            print(f"missing: {OUT}")
            return 1
        with open(path, encoding="utf-8") as handle:
            if handle.read() != text:
                print(f"edited: {OUT}")
                return 1
        return 0
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
