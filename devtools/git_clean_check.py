import subprocess
import sys
import os

def main():
    # Run git status --porcelain
    try:
        res = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True, check=True)
    except Exception as e:
        print(f"Error running git status: {e}", file=sys.stderr)
        sys.exit(1)

    lines = res.stdout.strip().split("\n")
    unexpected_files = []

    whitelist = ["HANDOFF.md"]

    for line in lines:
        if not line:
            continue
        # Format is XY path or XY "path"
        # XY is 2 chars
        if len(line) < 4:
            continue
        status_code = line[:2]
        file_path = line[3:].strip('"')
        
        # Check if file path is in whitelist or is within devtools/__pycache__
        normalized_path = os.path.normpath(file_path).replace("\\", "/")
        
        # Check whitelist
        is_whitelisted = False
        if normalized_path in whitelist:
            is_whitelisted = True
        elif normalized_path.startswith("devtools/__pycache__"):
            is_whitelisted = True
            
        if not is_whitelisted:
            unexpected_files.append((status_code, file_path))

    if unexpected_files:
        print("Git Clean Check FAILED: Found unexpected modified/staged/untracked files:")
        for status, path in unexpected_files:
            print(f"  {status} {path}")
        sys.exit(1)

    print("Git Clean Check PASSED: Repository is clean (except for whitelisted exceptions).")
    sys.exit(0)

if __name__ == '__main__':
    main()
