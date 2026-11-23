---
format: aep.planning-md/3
id: story:the-operator-runs-the-chat-bridge-on-the-real-channel
kind: story
status: draft
title: The operator runs the chat bridge once on the real chat channel and records it
summary: An operator run by a written check against the real chat provider, recorded as evidence; no part of the gate reaches the provider.
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
- depends_on: story:the-chat-bridge-asks-a-decision-once
scope:
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 6
---
## Outcome

The operator runs the chat bridge once against the real chat channel, through `connectors`, by
a written check, and records what was seen as evidence: a decision posted to the inbox of the
person appears in the chat channel, the person's thread reply answers it, and a decision left
without a reply is answered by silence. This is the run that shows the epic on the real chat
provider; it is an operator run, never a test of the gate.

Not in this set: the epic says the bridge "replaces a per-session poller of the chat channel"
(epic:human-inbox, Context). Nothing in this repository names that poller, so no story here
retires it (Open 3).

## Acceptance

A decision posted to the inbox of the person during the operator's run on the real chat channel
is moved from `Read` to `Answered` with `answered_by: principal` by the person's thread reply,
and the run is recorded as one evidence record of this story, where before this story the
bridge was run only against a stand-in of `connectors` (state transition: no recorded run on the
chat provider, to one recorded run in which the epic's acceptance was seen).

Checks, each of which can fail:

1. The check is written in `docs/design/bridge.md` (a section "The operator's check"), as
   `docs/design/wake.md` writes one for `claude_code`: each step with its command, the keys made
   for the run, the binding given as placeholders; no name of a chat workspace, a chat channel
   or a chat user, no token, no user id and no key in it.
2. What the operator sets up before the run is listed in the check and is done by the operator,
   not by the bridge or a task: the connection of `connectors` to the chat provider, the write
   policy or the proofs of Open 5 of story:the-chat-bridge-posts-a-pending-message, the key file
   with the key (and the delegation, under the operator's answer B in
   approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner;
   story:the-chat-bridge-runs-with-its-own-key-under-a-delegation) of the
   bridge, and a restart of the service so it reads that key file.
3. The record is one evidence record of this story (`aep plan artifact evidence`, a kind from
   the closed list the command prints), with the day, the versions of `inbox` and `connectors`,
   and, as `docs/design/wake.md` does in "Seen on a real session, not verified", what the run
   saw and what it did not see (such as a reaction, a reply that matches no option, a restart of
   the service during the run).
4. No task, no test, no CI step and no gate reaches the chat provider or starts the real
   `connectors`: the gate runs only the stand-in cases of the other stories of the epic.
5. No file of the repository and no evidence record holds a name of the chat workspace, the
   chat channel or a chat user, a chat user id, a token or the text of a message other than the
   messages made for the run.
6. `website/docs/status.md` and `website/docs/limits.md` say that the bridge was run on one real
   chat channel, and what was not seen, in the words of the record.
7. The record says, each as seen or not seen: the decision appeared as one chat post; the
   sender's `inbox collect` printed `answered_by: principal` and the option of the reply; a
   second decision with `default_on_silence` and a short `expires`, left without a reply, was
   collected with `answered_by: silence` and its `default_on_silence` as `decided_option`. Under
   Open 2 A of story:the-chat-bridge-asks-a-decision-once only, also that its thread got the line
   naming the option taken on silence; under its C the record does not look for that line. A
   record that lacks one of these items, as seen or as not seen, fails the check.

## Context

The folder rules: the test step makes no call to a real API and no network request; a run
against the real chat workspace is an operator run recorded as evidence (the brief of this
stage). The form is that of the `claude_code` adapter: its gate holds the plugin against a
stand-in harness, and "The operator checks the acceptance on one real session once per release
and records it as evidence of the story" (`docs/design/wake.md`, "Seen on a real session, not
verified, and the operator's check").

The epic's acceptance names the chat channel: "A decision posted to a person's inbox appears
in the chat channel, and the person's thread reply moves the message from `Read` to `Answered`
with `answered_by: principal`" (epic:human-inbox). The stand-in cases of
story:the-chat-bridge-turns-a-thread-reply-into-an-answer and
story:the-chat-bridge-asks-a-decision-once hold the same against the interface of
`connectors`; only this run shows that the real connector answers as the stand-in does.

## Domain relations

None new: the run uses the nouns and relations of the other stories of the epic as they are
answered.

## Open

1. How often the check runs. (A) once for this story, and again once per release that changes
   the bridge, as the `claude_code` check; (B) once only. Default: A.
2. The `expires` of the decision left to silence. (A) a few minutes, so the run ends within
   about 10 minutes; (B) the default of 24 hours, the run read the next day. Default: A.
3. The per-session poller of the chat channel the epic says the bridge replaces. (A) the
   operator retires it outside this repository after the recorded run; (B) a story here, once the
   operator names where the poller is. Default: A.

## Scope

After story:the-chat-bridge-turns-a-thread-reply-into-an-answer and
story:the-chat-bridge-asks-a-decision-once (edges `depends_on`). No code.

- `docs/design/bridge.md`: the section of the operator's check: inferred
- `website/docs/status.md`, `website/docs/limits.md`: the lines on the real run: cited
- the evidence record of this story, written through `aep plan artifact evidence`: inferred
