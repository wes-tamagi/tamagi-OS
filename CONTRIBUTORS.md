# Who makes Tamagi

**Author and maintainer:** wes-tamagi — project direction, product decisions
and the start menu (`cosmic-ext-classic-menu`).

## AI contributors

Tamagi is written with the help of AI coding agents, under the author's
direction. They are contributors to the project and are recorded here and in
the commit messages.

- **Gemini (Google), through Antigravity** — contributor. Implemented the
  multi-distro development box, native gaming, the first complete translation
  and the performance tuning; reviews the project's code and proposes ideas.
  Its review of the `english-cli-minimal` branch found, among others, the flaw
  in the agent jail inside the development box and the folders missing from
  the virt-manager layer, both fixed from that review.
- **Claude (Anthropic), through Claude Code** — contributor. The single
  `tamagi` command, the lean image and catalog, project-jailed agents, themes
  that follow COSMIC, image rechunking, the Rust tools, the installer fixes
  and the test suite.

## How the work is recorded

- Commits written with an agent carry the agent's `Co-Authored-By:` line.
- Review findings that become a fix carry `Reviewed-by:` in the commit.
- Every change goes through the suite (`just test`) before reaching `main`;
  what was not tested is said in the commit message.
