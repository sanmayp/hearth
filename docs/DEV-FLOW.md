# Dev flow

Hearth ships in small lanes. One brief defines one lane. One branch
carries it. Small gates check each step. Nothing lands without proof.

## Lanes

- One brief per lane. The brief names the goal, the files, and the
  acceptance copy. It fits on one screen.
- One branch per lane. The branch starts from the main line.
- Lanes reply READY with branch, commit, gate output, and test counts.
- One controller merges, one lane at a time. Lanes never merge or push.

## Gates

- G0 (fast): format check, clippy with denied warnings, crate tests,
  UI checks for changed screens.
- G1 (merge): full workspace tests, route contract diff (breaking
  changes fail), migration dry run, UI build.
- G2 (deploy): canary smoke on health, login, and one read per module.

Zero tests run means the gate fails. Flaky tests quarantine in a day.

## Data rules

- Migrations expand, then migrate, then contract, across 2 releases.
  Never drop a column in the release that stops using it.
- Each module ships dark behind a flag. Settings switch it on.
  Off is the rollback.
- Redis holds cache and queues only. Postgres stays the record.

## Writing rules

- All prose uses short plain sentences. One idea each. Present tense.
  No idioms. Warnings come before actions. See [ASD-STE100](ASD-STE100.md).
