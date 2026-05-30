# Sprint Punch List — PrintProof3D

**Audit date:** 2026-05-30

This list is the dev team's actionable fixes for the **current sprint** to exit Stage 1 with a robust foundation.

---

## Must-fix (Blockers + Criticals)

| # | ID | Severity | Role | What to do | Size |
|---|---|---|---|---|---|
| 1 | TEST-001 | Blocker | Test | Define core `PrinterAdapter` and `ModelValidator` traits and implement base logic to replace empty stubs | L |
| 2 | ENG-001 | Critical | Engineering | Implement verification rules on deserializer or `validate()` methods to check profile parameter bounds | M |
| 3 | UX-001 | Critical | UI/UX | Implement command line argument parsing with clap and display usage/version helps (resolve QA-006 too) | M |
| 4 | TEST-002 | Critical | Test | Create `.github/workflows/ci.yml` file to run compile and workspace tests on every remote push | S |
| 5 | DOC-001 | Critical | Docs | Replace skeletal README with the drafted production-grade README documentation | S |
| 6 | DOC-002 | Critical | Docs | Deploy drafted ARCHITECTURE.md explaining workspace crate relations | S |
| 7 | DOC-003 | Critical | Docs | Deploy drafted USER_MANUAL.md guiding configuration formats | S |
| 8 | DOC-004 | Critical | Docs | Deploy drafted API_REFERENCE.md listing types, functions, and codes | S |

---

## Should-fix (high-leverage Majors)

| # | ID | Severity | Role | What to do | Size |
|---|---|---|---|---|---|
| 1 | TEST-003 | Major | Test | Load STL and G-code fixture files into parser tests to assert validation limits | M |
| 2 | UX-002 | Major | UI/UX | Refactor `BuildVolume` in core schemas to support tagged rectangular and cylindrical types | M |
| 3 | UX-005 | Major | UI/UX | Extend `IssueLocation` schema to support bounding boxes and triangle arrays instead of single coordinates | M |
| 4 | UX-006 | Major | UI/UX | Replace confusing `Difficulty` enum for material `warp_risk` with a `RiskLevel` (`low`, `medium`, `high`) | S |
| 5 | ENG-003 | Major | Engineering | Refactor schema generation out of Hermetic unit tests to separate xtask or generator script | S |

---

## Suggested sequencing

1. **Fix UX-002 & UX-005 & UX-006 (Schema refactoring):** Do this first because changes to the schema structs in `crates/core` will regenerate the schema files.
2. **Fix ENG-001 (Input Validation):** Add defensive range validation on profiles before exposing printability engines.
3. **Fix TEST-001 (Define Traits):** Establish trait interfaces before implementing parsers or adapters to prevent API coupling.
4. **Fix TEST-003 (Load Fixtures):** Hook fixtures into tests.
5. **Fix UX-001 (CLI clap parser):** Implement clap parser now that data types and parameters are finalized.
6. **Fix DOC-* (Docs deployment):** Overwrite/add all doc rewrites.
7. **Fix TEST-002 (CI setup):** Deploy workflow as the final action.

---

## Sign-off gate

- [ ] Core profile deserializer rejects physically invalid values (negative dimensions/temps).
- [ ] Circular build volume and cylindrical properties successfully parse in JSON schema checks.
- [ ] CLI displays standard clap `--help` menu and exits with code `0`.
- [ ] CLI exits with non-zero exit codes on invalid arguments.
- [ ] All 8 documentation drafts successfully deployed to the root repository.
- [ ] pre-push hook passes hermetically without filesystem errors.
