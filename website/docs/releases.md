---
title: Releases
description: Where the releases of the inbox are, and what the first one holds.
---

# Releases

Releases are listed at
[github.com/beyond10x/inbox/releases](https://github.com/beyond10x/inbox/releases), and each
one has a section in
[`CHANGELOG.md`](https://github.com/beyond10x/inbox/blob/main/CHANGELOG.md).

A release is an annotated version tag (`0.1.0`) on a `main` commit whose checks are green, and
a GitHub Release whose notes say what works and which limits are open. The steps are in
[`docs/releasing.md`](https://github.com/beyond10x/inbox/blob/main/docs/releasing.md).

**The first release has no binary.** There is no program to ship until the command-line client
and `inbox serve` exist (`story:cli-client`). No crate is published either: every crate of the
workspace is `publish = false`. A release marks a state of the source.
