import os
import sys
import subprocess

def run_watched_command(name, command_args, timeout):
    watchdog_path = os.path.normpath(os.path.join("devtools", "watchdog.py"))
    cmd = [sys.executable, watchdog_path, "--timeout", str(timeout), "--"] + command_args
    print(f"\n--- Running Check: {name} ---")
    print(f"Command: {' '.join(cmd)}")
    sys.stdout.flush()

    try:
        res = subprocess.run(cmd, text=True)
        return res.returncode == 0
    except Exception as e:
        print(f"Watchdog failed to launch command: {e}", file=sys.stderr)
        return False

def main():
    cli_bin = os.path.normpath(os.path.join("target", "release", "printproof3d.exe" if os.name == 'nt' else "printproof3d"))

    checks = [
        ("Documentation Policy Scan", [sys.executable, os.path.normpath(os.path.join("devtools", "docs_policy_check.py"))], 120),
        ("Cargo Format Check", ["cargo", "fmt", "--all", "--", "--check"], 120),
        ("Cargo Clippy Lints", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"], 600),
        ("Workspace Unit Tests", ["cargo", "test", "--workspace"], 600),
        ("Git Parity Status", ["git", "status", "--short", "--branch"], 120),
        ("Build CLI Binary", ["cargo", "build", "--release", "--bin", "printproof3d"], 300),
        ("Model Validation Smoke Test", [
            cli_bin, "validate-model",
            "--model", os.path.normpath(os.path.join("fixtures", "tetrahedron.stl")),
            "--printer", os.path.normpath(os.path.join("profiles", "prusa_mk4.json")),
            "--material", os.path.normpath(os.path.join("profiles", "pla.json"))
        ], 120),
        ("G-code Validation Smoke Test", [
            cli_bin, "validate-gcode",
            "--gcode", os.path.normpath(os.path.join("fixtures", "safe_print.gcode")),
            "--printer", os.path.normpath(os.path.join("profiles", "prusa_mk4.json")),
            "--material", os.path.normpath(os.path.join("profiles", "pla.json"))
        ], 120),
        ("Browser Acceptance Tests", [sys.executable, "-u", os.path.normpath(os.path.join("devtools", "acceptance_tests.py"))], 300)
    ]

    results = {}
    any_failed = False

    for name, cmd, timeout in checks:
        success = run_watched_command(name, cmd, timeout)
        results[name] = "PASS" if success else "FAIL"
        if not success:
            any_failed = True

    print("\n" + "="*60)
    print("PRINTPROOF3D AGENT HEALTH-CHECK SUMMARY")
    print("="*60)
    for name, status in results.items():
        print(f"{name:<35} : {status}")
    print("="*60)

    print("\n[DOCUMENTATION REFERENCE]")
    print("Please read the integration guide for details on REST, MCP, and SDK validation interfaces:")
    print("docs/AGENT_PRINTER_VALIDATION.md")

    print("\n[GIT STATUS REMINDER]")
    print("Note: HANDOFF.md is an untracked local handoff document and may remain untracked in the git tree.")

    # Run git status output directly to show user
    print("\nCurrent git branch status:")
    subprocess.run(["git", "status", "--short", "--branch"])

    if any_failed:
        print("\n[RESULT] Health check FAILED. One or more checks returned errors or timed out.", file=sys.stderr)
        sys.exit(1)

    print("\n[RESULT] Health check PASSED. Harness is ready for printer validation usage.")
    sys.exit(0)

if __name__ == '__main__':
    main()
