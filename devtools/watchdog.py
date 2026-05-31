import sys
import os
import subprocess
import threading
import time

def main():
    args = sys.argv[1:]
    timeout = 120
    cmd_args = []

    # Parse --timeout
    i = 0
    while i < len(args):
        if args[i] == '--timeout':
            if i + 1 < len(args):
                timeout = int(args[i+1])
                i += 2
            else:
                print("Error: --timeout requires a value", file=sys.stderr)
                sys.exit(1)
        elif args[i] == '--':
            cmd_args = args[i+1:]
            break
        else:
            cmd_args = args[i:]
            break

    if not cmd_args:
        print("Error: No command specified to run.", file=sys.stderr)
        sys.exit(1)

    print(f"[WATCHDOG] Running command: {' '.join(cmd_args)}")
    print(f"[WATCHDOG] Timeout configured: {timeout} seconds")
    sys.stdout.flush()

    try:
        proc = subprocess.Popen(
            cmd_args,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            universal_newlines=True
        )
    except Exception as e:
        print(f"[WATCHDOG FAILED] Failed to start command {' '.join(cmd_args)}: {e}", file=sys.stderr)
        sys.exit(1)

    output_lines = []

    # Thread function to read output lines
    def read_output():
        for line in iter(proc.stdout.readline, ''):
            sys.stdout.write(line)
            sys.stdout.flush()
            output_lines.append(line)
        proc.stdout.close()

    t = threading.Thread(target=read_output)
    t.daemon = True
    t.start()

    start_time = time.time()
    last_heartbeat = start_time

    while True:
        ret = proc.poll()
        if ret is not None:
            break

        current_time = time.time()
        elapsed = current_time - start_time

        if current_time - last_heartbeat >= 10.0:
            print(f"\n[WATCHDOG HEARTBEAT] elapsed: {int(elapsed)}s / {timeout}s")
            sys.stdout.flush()
            last_heartbeat = current_time

        if elapsed >= timeout:
            print(f"\n[WATCHDOG TIMEOUT] Command '{' '.join(cmd_args)}' exceeded timeout of {timeout} seconds.", file=sys.stderr)
            kill_process_tree(proc.pid)

            t.join(timeout=2)

            print("\n" + "="*60, file=sys.stderr)
            print("[WATCHDOG TIMEOUT ERROR SUMMARY]", file=sys.stderr)
            print(f"Command hung: {' '.join(cmd_args)}", file=sys.stderr)
            print(f"Total elapsed time: {int(elapsed)}s", file=sys.stderr)
            print("Last visible output:", file=sys.stderr)
            print("".join(output_lines[-20:]), file=sys.stderr)
            print("="*60, file=sys.stderr)
            sys.stderr.flush()

            print("[WATCHDOG RESULT] Verification FAILED due to timeout.", file=sys.stderr)
            sys.exit(1)

        time.sleep(0.1)

    t.join(timeout=5)

    if proc.returncode != 0:
        print(f"\n[WATCHDOG FAILURE] Command '{' '.join(cmd_args)}' failed with exit code {proc.returncode}.", file=sys.stderr)
        sys.exit(proc.returncode)

    print(f"\n[WATCHDOG SUCCESS] Command completed successfully in {int(time.time() - start_time)}s.")
    sys.exit(0)

def kill_process_tree(pid):
    if os.name == 'nt':
        try:
            subprocess.run(['taskkill', '/F', '/T', '/PID', str(pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception as e:
            print(f"[WATCHDOG] Failed taskkill tree-kill on PID {pid}: {e}", file=sys.stderr)
    else:
        import signal
        try:
            os.killpg(os.getpgid(pid), signal.SIGKILL)
        except Exception:
            try:
                os.kill(pid, signal.SIGKILL)
            except Exception:
                pass

if __name__ == '__main__':
    main()
