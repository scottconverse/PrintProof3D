import os
import sys

def get_all_doc_files(root_dir):
    doc_files = []
    exclude_dirs = {".git", "target", ".gemini", "devtools"}
    for dirpath, dirnames, filenames in os.walk(root_dir):
        # Filter excluded directories in-place to avoid traversing them
        dirnames[:] = [d for d in dirnames if d not in exclude_dirs]
        for filename in filenames:
            if filename.endswith(".md") or filename.endswith(".html"):
                doc_files.append(os.path.relpath(os.path.join(dirpath, filename), root_dir))
    return doc_files

# Literal forbidden phrases (case-insensitive)
LITERAL_PHRASES = [
    "safe to print",
    "hardware safety",
    "thermal runaway prevention",
    "print success",
    "known issue",
    "deferred",
    "next sprint",
    "watchlist",
    "secret_print_token",
]

# Semantic safety overclaims (case-insensitive)
SEMANTIC_OVERCLAIMS = [
    "ensure they are safe",
    "safety utility",
    "safe printing window",
    "protects your printer",
    "prevents hardware damage",
]

# Whitelist rules: (file_path_substring, line_number_1_indexed, phrase)
# Whitelisting "Deferred by Director" in docs/process/5-lens-self-audit.md
WHITELIST = [
    ("5-lens-self-audit.md", 29, "Deferred by Director")
]

def is_whitelisted(file_path, line_no, content):
    for wl_file, wl_line, wl_phrase in WHITELIST:
        if wl_file in file_path and line_no == wl_line:
            if wl_phrase in content:
                return True
    return False

def scan_file(file_path):
    violations = []
    if not os.path.exists(file_path):
        print(f"[Warning] File not found for scanning: {file_path}")
        return violations

    try:
        with open(file_path, "r", encoding="utf-8") as f:
            for idx, line in enumerate(f, 1):
                lower_line = line.lower()
                
                # 1. Check literal phrases
                for phrase in LITERAL_PHRASES:
                    if phrase in lower_line:
                        # Check if this occurrence is whitelisted
                        if not is_whitelisted(file_path, idx, line):
                            violations.append((idx, line.strip(), f"Literal: '{phrase}'"))

                # 2. Check semantic overclaims
                for phrase in SEMANTIC_OVERCLAIMS:
                    if phrase in lower_line:
                        if not is_whitelisted(file_path, idx, line):
                            violations.append((idx, line.strip(), f"Semantic Overclaim: '{phrase}'"))
    except Exception as e:
        print(f"[Error] Failed to read {file_path}: {e}")
        sys.exit(1)
        
    return violations

def main():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    print("=" * 60)
    print("RUNNING PRINTPROOF3D DOCUMENTATION COMPLIANCE SCAN")
    print("documentation-scoped: scans all repository .md and .html documentation surfaces, excluding implementation/tooling directories already excluded by the checker.")
    print("=" * 60)
    
    total_violations = 0
    target_files = get_all_doc_files(root_dir)
    for target in target_files:
        full_path = os.path.normpath(os.path.join(root_dir, target))
        violations = scan_file(full_path)
        
        rel_path = os.path.relpath(full_path, root_dir)
        if violations:
            print(f"\n[FAIL] {rel_path} - Found {len(violations)} violation(s):")
            for line_no, content, reason in violations:
                print(f"  Line {line_no:3d} | {reason:<30} | {content}")
            total_violations += len(violations)
        else:
            print(f"[PASS] {rel_path}")

    print("\n" + "=" * 60)
    if total_violations > 0:
        print(f"SCAN FAILED: Found {total_violations} total policy violations.")
        print("=" * 60)
        sys.exit(1)
    
    print("SCAN PASSED: All documentation files are compliant.")
    print("=" * 60)
    sys.exit(0)

if __name__ == "__main__":
    main()
