---
format: aep.planning-md/3
id: story:a-key-id-with-a-control-character-is-refused-where-it-is-made
kind: story
status: draft
title: A key id with a control character is refused where it is made
summary: inbox key new writes a key id with a line break; every client command then ends with exit 2 and a line that names no key file.
revision: 1
---
## Outcome

`inbox key new` refuses a key id that holds a control character, and a client command that
reads a secret key file whose `key_id` holds one ends as for every other defect of that file:
exit 4 and one line that names the file.

Today `inbox key new` writes such a file without complaint (exit 0). Since
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal the client makes no
request head with a line break in any of its parts, so every command with that file ends
with exit 2 and the line "a header value holds a line break; no request is sent", which names
neither the key file nor the key id (note 4 of
review-result:adversary-delegation-acts-pass-2). Before that story the client sent the head
with the line break in it.

## Acceptance

- `inbox key new` with a key id that holds a line break or another control character exits 2
  with one line that names the flag, and writes no file (state transition: a key id with a
  control character moves `inbox key new` from exit 0 to exit 2).
- A client command given a secret key file whose `key_id` holds a control character exits 4
  with one line that names the file, sends no request and opens no connection.
- The case of the adversary that holds the order of the request head and the connection
  (`adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`) uses such a key
  file today; it is given another part of a head that can hold a line break, or the story
  says why no such part is left and what holds the order then.
- Each new case takes under 5 s; `task check` exits 0.

## Context

A note of an adversary pass, filed as a draft. The key file is the caller's own, so no
boundary is crossed. It waits until the planned waves are delivered.
