"""Writes the authored scenarios of the inbox specification and the manifest that lists them.

usage: scenarios.py <ess directory> [--check]

It builds every scenario below and the manifest, runs every assertion, and only then writes
<ess directory>/scenarios/<name>.yaml for each scenario and <ess directory>/ess-inputs.yaml, so a
failed assertion changes no file. It removes no file: delete the file of a scenario taken out here.
With --check it writes nothing and exits 1 when a file differs from what it would write, printing
one line per path: `edited:`, `missing:`, or `not written by the script:` for a scenario file it
does not write.
COVERS maps a scenario name to the generated scenario ids it claims with `covers:`.
"""

import os
import re
import sys
from datetime import datetime, timedelta, timezone

import yaml

ESS = sys.argv[1]
CHECK = sys.argv[2:] == ["--check"]
assert sys.argv[2:] in ([], ["--check"]), __doc__
D = "inbox.messaging."


def out(command, outcome):
    return f"{D}{command}/outcome/{outcome}"


def tr(transition, command, outcome):
    return f"{D}Message/transition/{transition}/by/{D}{command}/{outcome}"


def st(state, command):
    return f"{D}Message/state/{state}/refuses/{D}{command}"


# A type's invariants read off a view field: `inv` for a field of that type, `wrapped` for one
# reached through an Optional, a struct or a list, which synthesis names by the digest of its path.
# Each tuple ends with the path a `contains` row must hold a value at for the claim to rest on it;
# a path runs through every element of a list. `reply` is claimed whole in a row: inbox-conformance
# does not read a dotted row path such as `reply.body`.
REPLY_BODY = ("Body", "reply", "6f225dcb81a1756258e5c80e589dcff62c29e6e19a7a5f6ccdce295dc91c5680",
              ("reply", "body"))
REPLY_ID = ("MessageId", "reply", "2a59efac4c3ac205ca937cd02a066886cbd43648835cc56ca35246776dabd9bb",
            ("reply", "reply_id"))
REPLY_OPTION = ("OptionLetter", "reply",
                "86a88b3f3da7e2830b7ed86ff27a235c8c2aa1b8b1ec906e031cf4f3f20680cc", ("reply", "option"))
OPTIONAL_LETTER = "5a5de3e7ecffe323b37c180756920d594f1df48d3d00a5289072b4349e37fef4"
DECIDED = ("OptionLetter", "decided_option", OPTIONAL_LETTER, ("decided_option",))
SILENCE = ("OptionLetter", "default_on_silence", OPTIONAL_LETTER, ("default_on_silence",))
LETTERS = ("OptionLetter", "decision", "5fc40af03c03b2190846fd86e4dc7b3694f364228b490834f133211ec5ee2e25",
           ("decision", "options", "letter"))
RECOMMENDED = ("OptionLetter", "decision",
               "c9e38459e74269aef672d56a96584e08784681471855f791bed107fca2f65578",
               ("decision", "recommended_option"))
WRAPPED = {(at[0], at[1], at[2]): at[3]
           for at in (REPLY_BODY, REPLY_ID, REPLY_OPTION, DECIDED, SILENCE, LETTERS, RECOMMENDED)}
INVARIANT_AT = re.compile(
    rf"^{re.escape(D)}(\w+)/invariant(-wrapped)?/at/{re.escape(D)}(\w+)/(\w+)(?:/([0-9a-f]{{64}}))?$")


def inv(type_, view, field):
    return f"{D}{type_}/invariant/at/{D}{view}/{field}"


def wrapped(view, at):
    type_, field, digest, _ = at
    return f"{D}{type_}/invariant-wrapped/at/{D}{view}/{field}/{digest}"


def reads(claim):
    """The view and the row path a claim of a type invariant at a view field rests on, read off the
    id itself, however the id was written; None for an id of another form."""
    if "/invariant" not in claim:
        return None
    match = INVARIANT_AT.match(claim)
    assert match, f"{claim}: an invariant id of a shape this script does not read"
    type_, is_wrapped, view, field, digest = match.groups()
    if not is_wrapped:
        assert digest is None, claim
        return view, (field,)
    assert (type_, field, digest) in WRAPPED, f"{claim}: a wrapped path this script does not know"
    return view, WRAPPED[(type_, field, digest)]


def values_at(value, path):
    """Every value a row holds at a path, through each element of a list; None where one is absent."""
    if not path:
        return [value]
    if isinstance(value, list):
        return [v for element in value for v in values_at(element, path)] if value else [None]
    if isinstance(value, dict):
        return values_at(value.get(path[0]), path[1:])
    return [None]


def envelope(view):
    return [inv("Body", view, "body"), inv("MessageId", view, "message_id"),
            inv("ThreadId", view, "thread"), wrapped(view, LETTERS), wrapped(view, RECOMMENDED),
            wrapped(view, SILENCE)]


def answer_fields(view):
    return [wrapped(view, at) for at in (REPLY_BODY, REPLY_ID, REPLY_OPTION, DECIDED)]


# The generated scenarios synthesis refuses on ess 0.97.1 that an authored scenario checks instead.
COVERS = {
    "post-read-answer": [
        out("MarkDelivered", "delivered"), out("MarkRead", "read"), out("Answer", "answered"),
        tr("deliver", "MarkDelivered", "delivered"), tr("read", "MarkRead", "read"),
        tr("answer", "Answer", "answered"),
    ],
    "handled-message-is-gone": [out("Collect", "collected")],
    "note-read-is-gone": [out("MarkRead", "handled")],
    "repeated-dedup-key": [out("PostMessage", "duplicate-key")],
    "repost-after-restart-arrives-once": [out("PostMessage", "duplicate")],
    "refused-reply-leaves-the-request-unanswered": [out("Answer", "shape-mismatch")],
    "decision-taken-on-silence": [
        out("Expire", "decided-on-silence"), tr("decide_on_silence", "Expire", "decided-on-silence"),
    ],
    "note-expires": [out("Expire", "expired"), out("Expire", "not-due")],
    "relayed-decision-without-delegation-refused": [out("PostMessage", "no-authority")],
    "only-the-owner-handles-its-inbox": [
        out("MarkDelivered", "not-recipient"), out("MarkRead", "not-recipient"),
        out("Answer", "not-recipient"), out("Refuse", "not-recipient"), out("Collect", "not-sender"),
        out("OpenInbox", "not-owner"),
    ],
    "refused-message-waits-for-the-sender": [
        out("Collect", "not-settled"), out("Refuse", "refused"), tr("refuse", "Refuse", "refused"),
        st("Sent", "Collect"), st("Refused", "Answer"), st("Refused", "Refuse"), st("Refused", "Expire"),
    ],
    "answered-decision-expires-uncollected": [st("Answered", "Expire")],
    "decision-answered-by-the-operator": [
        st("Answered", "Answer"), st("Answered", "Refuse"), *envelope("Settled"), *envelope("Replies"),
        *[i for view in ("Settled", "Replies", "Thread", "Outbox", "Messages", "Due")
          for i in answer_fields(view)],
    ],
    "closed-inbox-refuses-posts": [out("OpenInbox", "opened"), out("CloseInbox", "already-closed")],
    "envelope-must-fit-its-kind": [out("PostMessage", "request-without-shape")],
    "marks-repeat-without-effect": [
        out("MarkDelivered", "already-delivered"), out("MarkRead", "already-read"),
        st("Delivered", "Collect"), st("Read", "Collect"),
    ],
    "note-takes-no-answer": [out("Answer", "note")],
    "reply-envelope-is-checked": [
        out("Answer", "already-expired"), out("Answer", "no-authority"),
        out("Answer", "signature-invalid"), out("Answer", "authority-invalid"),
    ],
    "read-decision-waits-for-its-answer": envelope("Unanswered"),
    "answer-from-another-principal-refused": [out("Answer", "not-sender")],
    "a-third-principal-finds-no-message": [
        out(command, "foreign-message")
        for command in ("MarkDelivered", "MarkRead", "Answer", "Refuse", "Collect")
    ],
    "a-message-id-another-sender-holds-is-taken": [out("PostMessage", "id-taken")],
}

PLANNER = "agent:ws/planner"
WORKER = "agent:box/worker"
OTHER = "agent:box/other"
HOST_OPERATOR = "agent:ws/host-operator"
OPERATOR = "human:operator"


def ts(minutes):
    base = datetime(2026, 1, 5, 9, 0, 0, tzinfo=timezone.utc)
    return (base + timedelta(minutes=minutes)).strftime("%Y-%m-%dT%H:%M:%SZ")


def mid(code, n):
    out = f"01JGT0{code}{n:018d}"
    assert len(out) == 26, out
    assert all(c in "0123456789ABCDEFGHJKMNPQRSTVWXYZ" for c in out), out
    return out


def who(principal):
    return {"principal": principal, "inbox": principal}


class Scenario:
    def __init__(self, name, summary):
        self.name = name
        self.summary = summary
        assert len(summary) <= 200, (name, len(summary))
        self.steps = []
        self.asserts = []
        self.minute = 0

    def step(self, command, actor, caller, input_, outcome, error=None, events=None, no_events=None):
        step = {"at": ts(self.minute), "command": D + command, "actor": D + actor}
        self.minute += 1
        if caller is not None:
            step["caller"] = who(caller)
        step["input"] = input_
        step["outcome"] = outcome
        if error:
            step["error"] = {"name": D + error}
        if events:
            step["events"] = [{"event": D + name, "payload": payload} for name, payload in events]
        if no_events:
            step["no_events"] = [D + name for name in no_events]
        self.steps.append(step)
        return self

    def open(self, principal, outcome="opened", adapter="claude_code", caller=None, error=None):
        events = None
        if outcome in ("opened", "reopened"):
            events = [("InboxOpened", {"address": principal})]
        return self.step(
            "OpenInbox", "Client", caller or principal, {"address": principal, "adapter": adapter},
            outcome, error=error, events=events,
            no_events=["InboxOpened"] if error else None,
        )

    def close(self, principal, outcome="closed", caller=None, error=None):
        events = [("InboxClosed", {"address": principal})] if outcome == "closed" else None
        return self.step(
            "CloseInbox", "Client", caller or principal, {"address": principal}, outcome,
            error=error, events=events, no_events=["InboxClosed"] if error else None,
        )

    def post(self, message_id, to, kind="note", sender=PLANNER, outcome="posted", error=None,
             body="Build 512 is green.", created=0, expires=24 * 60, caller=None, **extra):
        """`sender` is the envelope's `from`; `caller`, the principal that sends the request, is
        the sender unless named."""
        input_ = {
            "message_id": message_id,
            "from": sender,
            "on_behalf_of": extra.pop("on_behalf_of", sender),
        }
        if "authority" in extra:
            input_["authority"] = extra.pop("authority")
        input_["to"] = to
        input_["kind"] = kind
        input_["thread"] = extra.pop("thread", message_id)
        for key in ("reply_shape", "decision", "default_on_silence"):
            if key in extra:
                input_[key] = extra.pop(key)
        input_["created"] = ts(created)
        input_["expires"] = ts(expires)
        input_["dedup_key"] = extra.pop("dedup_key", message_id)
        input_["body"] = body
        input_["signature"] = {"key_id": extra.pop("key_id", "key-1"), "value": "c2lnbmF0dXJl"}
        assert not extra, extra
        events = None
        if outcome == "posted":
            events = [("MessagePosted", {
                "message_id": message_id, "to": to, "from": sender,
                "thread": input_["thread"], "kind": kind,
            })]
        return self.step(
            "PostMessage", "Client", caller or sender, input_, outcome, error=error, events=events,
            no_events=["MessagePosted"] if error else None,
        )

    def mark(self, command, message_id, caller, outcome, error=None, events=None, no_events=None):
        return self.step(
            command, "Client", caller, {"message_id": message_id}, outcome, error=error,
            events=events, no_events=no_events,
        )

    def delivered(self, message_id, caller=WORKER, outcome="delivered", error=None):
        events = [("MessageDelivered", {"message_id": message_id})] if outcome == "delivered" else None
        return self.mark("MarkDelivered", message_id, caller, outcome, error, events,
                         None if events else ["MessageDelivered"])

    def read(self, message_id, caller=WORKER, outcome="read", error=None):
        events = None
        if outcome == "read":
            events = [("MessageRead", {"message_id": message_id})]
        if outcome == "handled":
            events = [("MessageHandled", {"message_id": message_id})]
        return self.mark("MarkRead", message_id, caller, outcome, error, events,
                         None if events else ["MessageRead", "MessageHandled"])

    def answer(self, message_id, reply_id, caller=WORKER, outcome="answered", error=None,
               body="It is in the delivery module.", sender=PLANNER, thread=None, created=5,
               expires=24 * 60 + 5, from_=None, **extra):
        """`caller` sends the request; the reply's `from` is the caller unless `from_` names
        another principal."""
        from_ = from_ or caller
        input_ = {
            "in_reply_to": message_id,
            "reply_id": reply_id,
            "from": from_,
            "on_behalf_of": extra.pop("on_behalf_of", from_),
        }
        if "authority" in extra:
            input_["authority"] = extra.pop("authority")
        input_["created"] = ts(created)
        input_["expires"] = ts(expires)
        if "option" in extra:
            input_["option"] = extra.pop("option")
        input_["body"] = body
        input_["signature"] = {"key_id": "key-2", "value": "c2lnbmF0dXJl"}
        assert not extra, extra
        events = None
        if outcome == "answered":
            events = [("MessageAnswered", {
                "message_id": message_id, "reply_id": reply_id, "to": sender,
                "thread": thread or message_id,
            })]
        return self.step(
            "Answer", "Client", caller, input_, outcome, error=error, events=events,
            no_events=["MessageAnswered"] if error else None,
        )

    def refuse(self, message_id, reason, caller=WORKER, outcome="refused", error=None,
               sender=PLANNER, thread=None):
        events = None
        if outcome == "refused":
            events = [("MessageRefused", {
                "message_id": message_id, "to": sender, "thread": thread or message_id,
            })]
        return self.step(
            "Refuse", "Client", caller, {"message_id": message_id, "reason": reason}, outcome,
            error=error, events=events, no_events=["MessageRefused"] if error else None,
        )

    def collect(self, message_id, caller=PLANNER, outcome="collected", error=None):
        events = [("MessageHandled", {"message_id": message_id})] if outcome == "collected" else None
        return self.mark("Collect", message_id, caller, outcome, error, events,
                         None if events else ["MessageHandled"])

    def expire(self, message_id, now, outcome, error=None, keep=24 * 60, sender=PLANNER, thread=None):
        events = None
        if outcome == "expired":
            events = [("MessageExpired", {"message_id": message_id})]
        if outcome == "decided-on-silence":
            events = [("DecisionTakenOnSilence", {
                "message_id": message_id, "to": sender, "thread": thread or message_id,
            })]
        return self.step(
            "Expire", "Sweeper", None,
            {"message_id": message_id, "now": ts(now), "keep_until": ts(now + keep)}, outcome,
            error=error, events=events,
            no_events=None if events else ["MessageExpired", "DecisionTakenOnSilence"],
        )

    def check(self, view, params, **expectation):
        entry = {"view": D + view, "params": params}
        entry.update(expectation)
        self.asserts.append(entry)
        return self

    def empty(self, view, params):
        return self.check(view, params, counts={"at_most": 0})

    def exactly(self, view, params, n):
        return self.check(view, params, counts={"at_least": n, "at_most": n})

    def write(self):
        document = {"type": "ess-scenario/7"}
        if COVERS.get(self.name):
            document["covers"] = sorted(COVERS[self.name])
        document["domain"] = "inbox.messaging"
        document["scenario"] = self.name
        document["summary"] = self.summary
        document["timeline"] = self.steps
        document["assert"] = self.asserts
        OUTPUTS[f"scenarios/{self.name}.yaml"] = yaml.safe_dump(
            document, sort_keys=False, default_flow_style=False, width=100, allow_unicode=True)
        ASSERTS[self.name] = self.asserts
        return self.name


ASSERTS = {}
# Every file the script writes, by its path under the ess directory; written only after the checks.
OUTPUTS = {}


def inbox(address):
    return {"inbox": address}


def sender(principal=PLANNER):
    return {"sender": principal}


def thread(thread_id):
    return {"thread": thread_id}


LINES_3 = {"shape": "lines", "value": 3}
LINES_1 = {"shape": "lines", "value": 1}
ASK = {
    "options": [
        {"letter": "A", "text": "Ship the release today."},
        {"letter": "B", "text": "Hold the release until Monday."},
    ],
    "recommended_option": "B",
}

names = []

# 1
m = mid("PA", 1)
r = mid("PA", 2)
s = Scenario("post-read-answer",
             "A request is posted to an open inbox, handed to the wake surface, read and answered. The answer waits on the request for the sender.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3, body="Which crate holds the retry rule?")
s.delivered(m).read(m).answer(m, r)
s.empty("Pending", inbox(WORKER)).empty("Unanswered", inbox(WORKER))
s.check("Settled", sender(), contains={"message_id": m, "state": "Answered", "answered_by": "principal",
                                       "expires": ts(24 * 60 + 5)})
s.exactly("Replies", thread(m), 1)
names.append(s.write())

# 2
m = mid("HG", 1)
r = mid("HG", 2)
s = Scenario("handled-message-is-gone",
             "The sender collects the answer to its request. The message is handled by that and the service holds it no longer: every later command on its id finds no message.")
s.open(WORKER).post(m, WORKER, "request", reply_shape={"shape": "free", "value": {}})
s.answer(m, r).collect(m)
s.collect(m, outcome="unknown-message", error="UnknownMessage")
s.read(m, outcome="unknown-message", error="UnknownMessage")
s.answer(m, mid("HG", 3), outcome="unknown-message", error="UnknownMessage")
s.empty("Outbox", sender()).empty("Thread", thread(m)).empty("Pending", inbox(WORKER))
names.append(s.write())

# 3
m = mid("NR", 1)
s = Scenario("note-read-is-gone",
             "A note is handled when the owner of the inbox reads it, and is removed at once.")
s.open(WORKER).post(m, WORKER, "note").delivered(m).read(m, outcome="handled")
s.read(m, outcome="unknown-message", error="UnknownMessage")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender())
names.append(s.write())

# 4
m = mid("DK", 1)
m2 = mid("DK", 2)
s = Scenario("repeated-dedup-key",
             "A second envelope with another id and the dedup key of a live message is refused as a duplicate and stores nothing.")
s.open(WORKER).post(m, WORKER, "note", dedup_key="deploy-42")
s.post(m2, WORKER, "note", dedup_key="deploy-42", outcome="duplicate-key", error="Duplicate")
s.exactly("Pending", inbox(WORKER), 1)
s.check("Pending", inbox(WORKER), contains={"message_id": m, "dedup_key": "deploy-42"})
s.check("Outbox", sender(), excludes={"message_id": m2})
names.append(s.write())

# 5
m = mid("RR", 1)
s = Scenario("repost-after-restart-arrives-once",
             "A restarted service holds nothing, as at the start of a scenario. The owner opens its inbox again and the sender reposts; a second repost of the id is a duplicate, so the message arrives once.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.post(m, WORKER, "request", reply_shape=LINES_3, outcome="duplicate", error="Duplicate")
s.exactly("Pending", inbox(WORKER), 1).exactly("Outbox", sender(), 1)
names.append(s.write())

# 6
m = mid("WS", 1)
s = Scenario("reply-of-the-wrong-shape-refused",
             "A request asks for a reply of one line. A reply of three lines is refused and changes nothing; a reply of one line is then accepted.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_1).read(m)
s.answer(m, mid("WS", 2), outcome="shape-mismatch", error="ShapeMismatch",
         body="First line.\nSecond line.\nThird line.")
s.answer(m, mid("WS", 3), body="One line.")
s.check("Settled", sender(), contains={"message_id": m, "state": "Answered", "answered_by": "principal"})
s.empty("Unanswered", inbox(WORKER))
names.append(s.write())

# 7
m = mid("WR", 1)
s = Scenario("refused-reply-leaves-the-request-unanswered",
             "A reply the service refuses for its shape leaves the request as it was: read, unanswered and with nothing for the sender to collect.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_1).read(m)
s.answer(m, mid("WR", 2), outcome="shape-mismatch", error="ShapeMismatch",
         body="First line.\nSecond line.")
s.check("Unanswered", inbox(WORKER), contains={"message_id": m, "state": "Read", "expires": ts(24 * 60)})
s.empty("Settled", sender()).empty("Replies", thread(m))
names.append(s.write())

# 8
m = mid("DS", 1)
s = Scenario("decision-taken-on-silence",
             "Nobody answers a decision before `expires`. The expiry task answers it by silence with the option the decision names for that, and the answer waits for the sender until `keep_until`.")
s.open(OPERATOR).post(m, OPERATOR, "decision", decision=ASK, default_on_silence="B", expires=60,
                      body="Ship release 4.2 today?")
s.delivered(m, caller=OPERATOR)
s.expire(m, 59, "not-due", error="NotDue")
s.expire(m, 60, "decided-on-silence")
s.check("Settled", sender(), contains={"message_id": m, "state": "Answered", "answered_by": "silence",
                                       "decided_option": "B", "expires": ts(60 + 24 * 60)})
s.exactly("Replies", thread(m), 1).empty("Pending", inbox(OPERATOR))
names.append(s.write())

# 9
m = mid("NX", 1)
s = Scenario("note-expires",
             "A note nobody reads before `expires` is removed by the expiry task. Before that instant the task is refused; after the removal every command on the id finds no message.")
s.open(WORKER).post(m, WORKER, "note", expires=30)
s.expire(m, 29, "not-due", error="NotDue")
s.expire(m, 30, "expired")
s.expire(m, 31, "unknown-message", error="UnknownMessage")
s.read(m, outcome="unknown-message", error="UnknownMessage")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender())
names.append(s.write())

# 10
m = mid("BS", 1)
s = Scenario("bad-signature-refused",
             "The SignatureVerifier port refuses the signature of a post. The post is refused, which tells the sender, and nothing is stored for anyone to act on.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3, outcome="signature-invalid",
                    error="SignatureInvalid")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender())
names.append(s.write())

# 11
m = mid("RD", 1)
s = Scenario("relayed-decision-with-delegation-accepted",
             "A session relays a decision of the operator and names a delegation the SignatureVerifier port accepts. The message is stored with `from`, `on_behalf_of` and `authority` for the recipient to read.")
s.open(WORKER).post(m, WORKER, "note", sender=HOST_OPERATOR, on_behalf_of=OPERATOR,
                    authority="delegation-7", body="Decision D4: option A. Proceed with the release.")
s.check("Pending", inbox(WORKER), contains={"message_id": m, "from": HOST_OPERATOR,
                                           "on_behalf_of": OPERATOR, "authority": "delegation-7"})
names.append(s.write())

# 12
m = mid("RN", 1)
m2 = mid("RN", 2)
s = Scenario("relayed-decision-without-delegation-refused",
             "A session relays a decision of the operator and names no delegation: refused. It then names a delegation the SignatureVerifier port does not accept: refused. Nothing is stored either time.")
s.open(WORKER).post(m, WORKER, "note", sender=HOST_OPERATOR, on_behalf_of=OPERATOR,
                    outcome="no-authority", error="NoAuthority",
                    body="Decision D4: option A. Proceed with the release.")
s.post(m2, WORKER, "note", sender=HOST_OPERATOR, on_behalf_of=OPERATOR, authority="delegation-lapsed",
       outcome="authority-invalid", error="NoAuthority",
       body="Decision D4: option A. Proceed with the release.")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender(HOST_OPERATOR))
names.append(s.write())

# 13
workers = [f"agent:box/worker-{n:02d}" for n in range(1, 12)]
ids = [mid("FN", n) for n in range(1, 12)]
t = ids[0]
s = Scenario("request-fanned-out-to-eleven",
             "A sender asks 11 recipients one question: 11 envelopes under one thread. Three answer and one refuses. The Thread view holds 11 rows, one per recipient, and the Replies view the 3 answers.")
for w in workers:
    s.open(w)
for w, i in zip(workers, ids):
    s.post(i, w, "request", thread=t, reply_shape=LINES_3, body="Is your worktree clean?")
for n in range(3):
    s.answer(ids[n], mid("FN", 100 + n), caller=workers[n], thread=t, body="Yes.")
s.refuse(ids[3], "This session has no worktree.", caller=workers[3], thread=t)
s.exactly("Thread", thread(t), 11).exactly("Replies", thread(t), 3).exactly("Settled", sender(), 4)
s.check("Replies", thread(t), contains={"message_id": ids[2], "to": workers[2], "state": "Answered"})
s.check("Thread", thread(t), contains={"message_id": ids[3], "state": "Refused",
                                      "refusal": "This session has no worktree."})
s.exactly("Pending", inbox(workers[4]), 1).empty("Pending", inbox(workers[0]))
names.append(s.write())

# 14
m = mid("NP", 1)
s = Scenario("only-the-owner-handles-its-inbox",
             "Only the owner of an inbox marks, reads, answers or refuses its messages and opens or closes it, and only the sender collects. The sender, the owner or another caller is refused and nothing moves.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.delivered(m, caller=PLANNER, outcome="not-recipient", error="NotRecipient")
s.read(m, caller=PLANNER, outcome="not-recipient", error="NotRecipient")
s.answer(m, mid("NP", 2), caller=PLANNER, outcome="not-recipient", error="NotRecipient")
s.refuse(m, "Not mine.", caller=PLANNER, outcome="not-recipient", error="NotRecipient")
s.collect(m, caller=WORKER, outcome="not-sender", error="NotSender")
s.open(WORKER, caller=OTHER, outcome="not-owner", error="NotRecipient")
s.close(WORKER, caller=OTHER, outcome="not-owner", error="NotRecipient")
s.check("Pending", inbox(WORKER), contains={"message_id": m, "state": "Sent"})
s.check("Inboxes", {}, contains={"address": WORKER, "owner": WORKER, "state": "Open"})
names.append(s.write())

# 15
m = mid("RC", 1)
s = Scenario("refused-message-waits-for-the-sender",
             "The owner of an inbox refuses a request and says why. The sender cannot collect before that; afterwards the refusal waits on the message, and an answer is refused because the message is settled.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.collect(m, outcome="not-settled", error="NotSettled")
s.refuse(m, "Outside this session's repository.")
s.answer(m, mid("RC", 2), outcome="already-settled", error="AlreadySettled")
s.refuse(m, "Again.", outcome="already-settled", error="AlreadySettled")
s.expire(m, 60, "not-due", error="NotDue")
s.check("Settled", sender(), contains={"message_id": m, "state": "Refused",
                                       "refusal": "Outside this session's repository."})
s.empty("Pending", inbox(WORKER)).empty("Replies", thread(m))
names.append(s.write())

# 16
m = mid("RX", 1)
s = Scenario("refusal-collected-is-gone",
             "The sender collects the refusal of its request; the message is handled by that and removed.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.refuse(m, "Outside this session's repository.").collect(m)
s.collect(m, outcome="unknown-message", error="UnknownMessage")
s.empty("Outbox", sender()).empty("Settled", sender())
names.append(s.write())

# 17
m = mid("DA", 1)
s = Scenario("answered-decision-expires-uncollected",
             "An answer carries its own `expires`, which replaces the decision's. The sender never collects it, and the expiry task removes the message at that instant and not before.")
s.open(OPERATOR).post(m, OPERATOR, "decision", decision=ASK, default_on_silence="B", expires=60,
                      body="Ship release 4.2 today?")
s.read(m, caller=OPERATOR)
s.answer(m, mid("DA", 2), caller=OPERATOR, option="A", body="A. Ship it.", created=10, expires=120)
s.expire(m, 60, "not-due", error="NotDue")
s.expire(m, 120, "expired")
s.empty("Outbox", sender()).empty("Replies", thread(m))
names.append(s.write())

# 18
m = mid("DC", 1)
t = mid("DC", 9)
s = Scenario("decision-answered-by-the-operator",
             "The operator answers a decision with option A before `expires`. Every view that holds the message shows the reply, its option, and that a principal answered, not silence.")
s.open(OPERATOR).post(m, OPERATOR, "decision", thread=t, decision=ASK, default_on_silence="B",
                      expires=60, body="Ship release 4.2 today?")
s.answer(m, mid("DC", 2), caller=OPERATOR, option="A", body="A. Ship it.", created=10, expires=120,
         thread=t)
s.answer(m, mid("DC", 3), caller=OPERATOR, option="B", body="B. Hold it.", created=11, expires=121,
         outcome="already-settled", error="AlreadySettled")
s.refuse(m, "Too late.", caller=OPERATOR, outcome="already-settled", error="AlreadySettled")
reply = {"reply_id": mid("DC", 2), "from": OPERATOR, "on_behalf_of": OPERATOR, "created": ts(10),
         "option": "A", "body": "A. Ship it.", "signature": {"key_id": "key-2", "value": "c2lnbmF0dXJl"}}
answered = {"message_id": m, "thread": t, "state": "Answered", "answered_by": "principal",
            "decided_option": "A", "expires": ts(120), "reply": reply}
asked = {"body": "Ship release 4.2 today?", "decision": ASK, "default_on_silence": "B"}
s.check("Settled", sender(), contains={**answered, **asked})
s.check("Replies", thread(t), contains={**answered, **asked})
s.check("Thread", thread(t), contains=answered).check("Outbox", sender(), contains=answered)
s.check("Messages", {}, contains=answered).check("Due", {"now": ts(120)}, contains=answered)
s.empty("Due", {"now": ts(119)})
names.append(s.write())

# 19
m = mid("CX", 1)
m2 = mid("CX", 2)
m3 = mid("CX", 3)
s = Scenario("closed-inbox-refuses-posts",
             "A post to a closed inbox and a post to an address nobody opened are refused alike. The owner opens the inbox again and the next post is stored.")
s.open(WORKER).open(WORKER, outcome="already-open").close(WORKER).close(WORKER, outcome="already-closed")
s.post(m, WORKER, "note", outcome="inbox-closed", error="UnknownInbox")
s.post(m2, "agent:box/nobody", "note", outcome="unknown-inbox", error="UnknownInbox")
s.close("agent:box/nobody", outcome="unknown-inbox", error="UnknownInbox")
s.open(WORKER, outcome="reopened", adapter="terminal").post(m3, WORKER, "note")
s.check("Inboxes", {}, contains={"address": WORKER, "state": "Open", "adapter": "terminal"})
s.exactly("Pending", inbox(WORKER), 1)
s.check("Pending", inbox(WORKER), contains={"message_id": m3})
names.append(s.write())

# 20
s = Scenario("envelope-must-fit-its-kind",
             "A request without a reply shape, a decision without options or with a silence option it does not offer, and an envelope that expires no later than its creation are refused and store nothing.")
s.open(WORKER)
s.post(mid("EK", 1), WORKER, "request", outcome="request-without-shape", error="ShapeMismatch")
s.post(mid("EK", 2), WORKER, "decision", default_on_silence="B", outcome="decision-without-options",
       error="ShapeMismatch")
s.post(mid("EK", 3), WORKER, "decision", decision=ASK, default_on_silence="C", outcome="unknown-option",
       error="ShapeMismatch")
s.post(mid("EK", 4), WORKER, "note", created=10, expires=10, outcome="already-expired",
       error="AlreadyExpired")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender())
names.append(s.write())

# 21
m = mid("TW", 1)
s = Scenario("marks-repeat-without-effect",
             "An adapter that reports a delivery twice and an owner that reads twice change nothing the second time, and a delivery reported after the read does not take the message back.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.delivered(m).delivered(m, outcome="already-delivered")
s.collect(m, outcome="not-settled", error="NotSettled")
s.read(m).read(m, outcome="already-read").delivered(m, outcome="already-delivered")
s.collect(m, outcome="not-settled", error="NotSettled")
s.check("Unanswered", inbox(WORKER), contains={"message_id": m, "state": "Read"})
s.empty("Pending", inbox(WORKER))
names.append(s.write())

# 22
m = mid("AN", 1)
s = Scenario("note-takes-no-answer",
             "A note asks for nothing: an answer to it is refused and the note stays for its owner to read.")
s.open(WORKER).post(m, WORKER, "note")
s.answer(m, mid("AN", 2), outcome="note", error="NotAnswerable")
s.check("Pending", inbox(WORKER), contains={"message_id": m, "state": "Sent", "kind": "note"})
names.append(s.write())

# 23
m = mid("RE", 1)
s = Scenario("reply-envelope-is-checked",
             "The service checks a reply envelope as it checks a post: its lifetime, its signature and its authority. A refused reply leaves the request waiting for an answer.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.answer(m, mid("RE", 2), created=10, expires=10, outcome="already-expired", error="AlreadyExpired")
s.answer(m, mid("RE", 3), on_behalf_of=OPERATOR, outcome="no-authority", error="NoAuthority")
s.answer(m, mid("RE", 4), outcome="signature-invalid", error="SignatureInvalid")
s.answer(m, mid("RE", 5), on_behalf_of=OPERATOR, authority="delegation-lapsed",
         outcome="authority-invalid", error="NoAuthority")
s.check("Pending", inbox(WORKER), contains={"message_id": m, "state": "Sent", "expires": ts(24 * 60)})
s.empty("Settled", sender())
names.append(s.write())

# 24
m = mid("RW", 1)
t = mid("RW", 9)
s = Scenario("read-decision-waits-for-its-answer",
             "The operator reads a decision and does not answer it yet. It waits in Unanswered with its body, its thread, its options, the recommended option and the option silence takes.")
s.open(OPERATOR).post(m, OPERATOR, "decision", thread=t, decision=ASK, default_on_silence="B",
                      expires=60, body="Ship release 4.2 today?")
s.read(m, caller=OPERATOR)
s.check("Unanswered", inbox(OPERATOR), contains={
    "message_id": m, "thread": t, "state": "Read", "body": "Ship release 4.2 today?",
    "decision": ASK, "default_on_silence": "B",
})
s.empty("Pending", inbox(OPERATOR))
names.append(s.write())

# 25
m = mid("FC", 1)
s = Scenario("post-from-another-principal-refused",
             "The recipient of a note posts the sender's signed envelope again under its own request, while the note is live and after it is handled: refused both times, and nothing is stored.")
s.open(WORKER).post(m, WORKER, "note")
s.post(m, WORKER, "note", caller=WORKER, outcome="not-sender", error="NotSender")
s.read(m, outcome="handled")
s.post(m, WORKER, "note", caller=WORKER, outcome="not-sender", error="NotSender")
s.empty("Pending", inbox(WORKER)).empty("Outbox", sender())
names.append(s.write())

# 26
m = mid("FA", 1)
s = Scenario("answer-from-another-principal-refused",
             "A reply whose `from` is not the caller is refused, from another principal naming the owner and from the owner naming another principal; the request stays unanswered.")
s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.answer(m, mid("FA", 2), caller=OTHER, from_=WORKER, outcome="not-sender", error="NotSender")
s.answer(m, mid("FA", 3), caller=WORKER, from_=OTHER, outcome="not-sender", error="NotSender")
s.check("Pending", inbox(WORKER), contains={"message_id": m, "state": "Sent", "expires": ts(24 * 60)})
s.empty("Settled", sender()).empty("Replies", thread(m))
names.append(s.write())

# 27
m = mid("TP", 1)
m2 = mid("TP", 2)
m3 = mid("TP", 3)
s = Scenario("a-third-principal-finds-no-message",
             "A caller that is neither the sender nor the recipient is answered as for no message in every state a message is in, and moves nothing; once it is collected or expired, there is none.")
replies = iter(range(100, 200))


def third(message_id, outcome="foreign-message"):
    s.delivered(message_id, caller=OTHER, outcome=outcome, error="UnknownMessage")
    s.read(message_id, caller=OTHER, outcome=outcome, error="UnknownMessage")
    s.answer(message_id, mid("TP", next(replies)), caller=OTHER, outcome=outcome,
             error="UnknownMessage")
    s.refuse(message_id, "Not mine.", caller=OTHER, outcome=outcome, error="UnknownMessage")
    s.collect(message_id, caller=OTHER, outcome=outcome, error="UnknownMessage")


s.open(WORKER).post(m, WORKER, "request", reply_shape=LINES_3)
s.post(m2, WORKER, "request", reply_shape=LINES_3).post(m3, WORKER, "note")
third(m)
s.delivered(m)
third(m)
s.read(m)
third(m)
s.answer(m, mid("TP", 4))
third(m)
s.refuse(m2, "Outside this session's repository.")
third(m2)
s.collect(m)
third(m, "unknown-message")
third(m3)
s.expire(m3, 24 * 60, "expired")
third(m3, "unknown-message")
s.exactly("Settled", sender(), 1)
s.check("Settled", sender(), contains={"message_id": m2, "state": "Refused",
                                       "refusal": "Outside this session's repository."})
s.empty("Pending", inbox(WORKER)).empty("Replies", thread(m))
names.append(s.write())

# 28
m = mid("KS", 1)
m2 = mid("KS", 2)
m3 = mid("KS", 3)
m4 = mid("KS", 4)
key = "deploy-2026-11-21"
s = Scenario("dedup-key-is-the-senders",
             "A dedup key refuses only a second post of its own sender, to any inbox. A third principal's key in its own inbox refuses no post of another sender.")
s.open(WORKER).open(OTHER)
s.post(m, OTHER, "note", sender=OTHER, dedup_key=key)
s.post(m2, WORKER, "note", dedup_key=key)
s.post(m3, WORKER, "note", dedup_key=key, outcome="duplicate-key", error="Duplicate")
s.post(m4, OTHER, "note", dedup_key=key, outcome="duplicate-key", error="Duplicate")
s.exactly("Pending", inbox(OTHER), 1)
s.check("Pending", inbox(OTHER), contains={"message_id": m, "from": OTHER, "dedup_key": key})
s.exactly("Pending", inbox(WORKER), 1)
s.check("Pending", inbox(WORKER), contains={"message_id": m2, "from": PLANNER, "dedup_key": key})
s.exactly("Outbox", sender(), 1)
s.check("Outbox", sender(), excludes={"message_id": m3})
s.check("Outbox", sender(), excludes={"message_id": m4})
names.append(s.write())

# 29
m = mid("TQ", 1)
m2 = mid("TQ", 2)
s = Scenario("a-third-principal-cannot-tell-a-message-id-by-post",
             "A principal posts under the id of a message between two others and under a free id, to an address no inbox has and into its own closed inbox: answered alike, nothing stored.")
s.open(WORKER).open(OTHER).post(m, WORKER, "note")
s.post(m, "agent:box/nobody", "note", sender=OTHER, body="Probe.", outcome="unknown-inbox",
       error="UnknownInbox")
s.post(m2, "agent:box/nobody", "note", sender=OTHER, body="Probe.", outcome="unknown-inbox",
       error="UnknownInbox")
s.post(m, "agent:box/nobody", "note", outcome="unknown-inbox", error="UnknownInbox")
s.close(OTHER)
s.post(m, OTHER, "note", sender=OTHER, body="Probe.", outcome="inbox-closed", error="UnknownInbox")
s.post(m2, OTHER, "note", sender=OTHER, body="Probe.", outcome="inbox-closed", error="UnknownInbox")
s.exactly("Pending", inbox(WORKER), 1)
s.check("Pending", inbox(WORKER), contains={"message_id": m, "from": PLANNER, "body": "Build 512 is green."})
s.empty("Pending", inbox(OTHER)).empty("Outbox", sender(OTHER)).exactly("Outbox", sender(), 1)
names.append(s.write())

# 30
m = mid("QT", 1)
s = Scenario("a-message-id-another-sender-holds-is-taken",
             "A post under the id of another sender's live message is refused as taken and stores nothing. The sender's own repost is still a duplicate, also after the recipient closed the inbox.")
s.open(WORKER).open(OTHER).post(m, WORKER, "note")
s.post(m, OTHER, "note", sender=OTHER, body="Taken.", outcome="id-taken", error="MessageIdTaken")
s.post(m, WORKER, "note", sender=OTHER, body="Taken.", outcome="id-taken", error="MessageIdTaken")
s.post(m, WORKER, "note", outcome="duplicate", error="Duplicate")
s.close(WORKER)
s.post(m, WORKER, "note", outcome="duplicate", error="Duplicate")
s.exactly("Pending", inbox(WORKER), 1)
s.check("Pending", inbox(WORKER), contains={"message_id": m, "from": PLANNER, "body": "Build 512 is green."})
s.empty("Pending", inbox(OTHER)).empty("Outbox", sender(OTHER))
names.append(s.write())

for name in COVERS:
    assert name in names, name
claimed = [i for ids_ in COVERS.values() for i in ids_]
assert len(claimed) == len(set(claimed)), "an id is claimed twice"
# A claim of a type invariant at a view field rests on a row its scenario reads in that view with a
# value at the field's path: every value there, through each list element, is present and not null.
for name, ids_ in COVERS.items():
    for claim in ids_:
        read = reads(claim)
        if read is None:
            continue
        view, path = read
        rows = [a["contains"] for a in ASSERTS[name] if a["view"] == D + view and "contains" in a]
        assert any(None not in values_at(row, path) for row in rows), \
            f"{name} claims {claim} and reads no value at `{'.'.join(path)}` in a row of {view}"

manifest = {
    "format": "ess-inputs/2",
    "requires": "ess 0.97.1",
    "specification": ["system.yaml", "components.yaml", "domains/messaging.yaml"],
    "scenarios": [f"scenarios/{name}.yaml" for name in sorted(names)],
}
OUTPUTS["ess-inputs.yaml"] = yaml.safe_dump(manifest, sort_keys=False, default_flow_style=False)

if CHECK:
    drift = []
    for rel, text in sorted(OUTPUTS.items()):
        path = os.path.join(ESS, rel)
        if not os.path.exists(path):
            drift.append(f"missing: {path}")
        elif open(path, encoding="utf-8").read() != text:
            drift.append(f"edited: {path}")
    for entry in sorted(os.listdir(os.path.join(ESS, "scenarios"))):
        if f"scenarios/{entry}" not in OUTPUTS:
            drift.append(f"not written by the script: {os.path.join(ESS, 'scenarios', entry)}")
    for line in drift:
        print(line)
    print(len(names), "scenarios,", len(drift), "path(s) differ from tools/scenarios.py")
    sys.exit(1 if drift else 0)

for rel, text in OUTPUTS.items():
    with open(os.path.join(ESS, rel), "w", encoding="utf-8") as out:
        out.write(text)
print(len(names), "scenarios")
