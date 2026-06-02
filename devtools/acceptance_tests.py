import os
import sys
import socket
import subprocess
import time
import json
from playwright.sync_api import sync_playwright

def find_free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(('127.0.0.1', 0))
    port = s.getsockname()[1]
    s.close()
    return port

def main():
    print("=== STARTING PRINTPROOF3D BROWSER ACCEPTANCE TESTS ===")
    
    port = find_free_port()
    print(f"Allocated ephemeral port: {port}")
    
    # 1. Start REST server as a subprocess
    rest_env = os.environ.copy()
    rest_env["PRINTPROOF3D_PORT"] = str(port)
    
    # Run server via cargo run inside crates/rest
    rest_dir = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "crates", "rest"))
    cmd = ["cargo", "run", "--bin", "printproof3d-rest"]
    print(f"Launching REST server via: {' '.join(cmd)}")
    
    server_proc = subprocess.Popen(
        cmd,
        cwd=os.path.normpath(os.path.join(os.path.dirname(__file__), "..")),
        env=rest_env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1
    )
    
    # 2. Wait for startup stdout logs (Listening address and Ephemeral token)
    token = None
    startup_done = False
    
    time_limit = time.time() + 60
    while time.time() < time_limit:
        line = server_proc.stdout.readline()
        if not line:
            break
        print(f"[Server Output] {line.strip()}")
        if "Listening on" in line:
            startup_done = True
        if "Ephemeral token generated:" in line:
            token = line.split("Ephemeral token generated:")[-1].strip()
        if startup_done and (token is not None or "PRINTPROOF3D_API_TOKEN" in os.environ):
            break
        time.sleep(0.1)

    if "PRINTPROOF3D_API_TOKEN" in os.environ:
        token = os.environ["PRINTPROOF3D_API_TOKEN"]

    if not startup_done or token is None:
        print("Error: REST Server failed to start or print ephemeral token in time.", file=sys.stderr)
        if server_proc.poll() is not None:
            print(f"Process terminated with exit code: {server_proc.returncode}", file=sys.stderr)
        server_proc.terminate()
        sys.exit(1)

    print(f"Discovered server auth token: {token}")
    
    # Give the server an extra second to bind
    time.sleep(1)
    
    # 3. Setup Playwright tests
    base_url = f"http://127.0.0.1:{port}"
    tests_failed = False
    
    # Try all browsers in playwright
    browsers_to_try = ["chromium", "firefox", "webkit"]
    tested_any = False
    
    with sync_playwright() as p:
        for browser_name in browsers_to_try:
            print(f"\n--- Testing Browser: {browser_name} ---")
            try:
                browser_launcher = getattr(p, browser_name)
                # Headless mode for CI
                browser = browser_launcher.launch(headless=True)
                tested_any = True
            except Exception as e:
                print(f"Skipping browser {browser_name}: Not installed or failed to launch. Details: {e}")
                continue

            try:
                # Set up page context
                context = browser.new_context(viewport={"width": 1280, "height": 800})
                page = context.new_page()

                # Navigate to dashboard
                print(f"Navigating to {base_url}/ ...")
                page.goto(f"{base_url}/")
                assert "PrintProof3D — Print Validation Dashboard" in page.title(), "Title mismatch"

                # Check Predefined Profiles are populated
                print("Checking predefined profiles dropdown list...")
                page.wait_for_function("document.querySelectorAll('#select-printer option').length > 1", timeout=10000)
                page.wait_for_function("document.querySelectorAll('#select-material option').length > 1", timeout=10000)
                
                printer_opts = page.locator("#select-printer option").all()
                assert len(printer_opts) > 1, "Predefined printers not loaded"
                
                material_opts = page.locator("#select-material option").all()
                assert len(material_opts) > 1, "Predefined materials not loaded"

                # Test 1: Authentication Rejection
                print("Testing auth rejection (empty token)...")
                # Set input files
                tetra_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "fixtures", "tetrahedron.stl"))
                page.set_input_files("#input-file", tetra_path)
                
                # Select Prusa MK4 printer & PLA material
                page.select_option("#select-printer", label="Prusa MK4")
                page.select_option("#select-material", label="Polylactic Acid")
                
                # Click Validate
                page.click("#btn-validate")
                page.wait_for_timeout(1000)
                
                # Assert status displays "fail" and issues contains UNAUTHORIZED
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "fail", f"Expected status 'fail' on auth rejection, got '{status_text}'"
                
                issues_text = page.locator("#issues-list").inner_text().strip()
                assert "UNAUTHORIZED_ACCESS" in issues_text, "Auth rejection error not rendered in issues list"

                # Test 2: Authentication Success & STL Upload
                print("Testing auth success and model validation...")
                page.fill("#input-api-token", token)
                page.click("#btn-validate")
                
                # Wait for validation response
                page.wait_for_function("document.getElementById('status-display').innerText.toLowerCase() === 'pass'", timeout=10000)
                
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "pass", f"Expected pass status, got '{status_text}'"
                
                # Assert 3D Canvas is not blank by evaluating WebGL pixels
                print("Evaluating WebGL pixel buffer to verify non-blank canvas...")
                is_blank = page.evaluate("""() => {
                    const canvas = document.querySelector('canvas');
                    if (!canvas) return true;
                    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
                    if (!gl) return true;
                    const pixels = new Uint8Array(4);
                    gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                    return pixels[0] === 0 && pixels[1] === 0 && pixels[2] === 0 && pixels[3] === 0;
                }""")
                assert not is_blank, "Three.js WebGL canvas is blank (all pixel color values are zero)"

                # Test 3: Model Overhang Warning Validation
                print("Testing overhang model validation failure & issue details...")
                overhang_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "fixtures", "overhang_flange.stl"))
                page.set_input_files("#input-file", overhang_path)
                page.click("#btn-validate")
                
                page.wait_for_function("document.getElementById('status-display').innerText.toLowerCase() === 'warning'", timeout=10000)
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "warning", f"Expected warning status, got '{status_text}'"

                issues_text = page.locator("#issues-list").inner_text().strip()
                assert "OVERHANG_UNSUPPORTED" in issues_text, "Overhang issue code not rendered"
                assert "POOR_BED_ADHESION" in issues_text, "Bed adhesion issue code not rendered"

                # Test 4: G-code Path Validation
                print("Testing G-code path validation...")
                gcode_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "fixtures", "safe_print.gcode"))
                page.set_input_files("#input-file", gcode_path)
                page.click("#btn-validate")
                
                page.wait_for_function("document.getElementById('status-display').innerText.toLowerCase() === 'pass'", timeout=10000)
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "pass", f"Expected G-code validation to pass, got '{status_text}'"

                # Test 5: Custom Profile Upload Validation
                print("Testing custom profile uploads...")
                page.set_input_files("#input-file", tetra_path)
                
                # Setup custom printer profile upload
                page.select_option("#select-printer", value="custom")
                printer_json_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "profiles", "prusa_mk4.json"))
                page.set_input_files("#input-custom-printer", printer_json_path)

                # Setup custom material profile upload
                page.select_option("#select-material", value="custom")
                material_json_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "profiles", "pla.json"))
                page.set_input_files("#input-custom-material", material_json_path)

                page.click("#btn-validate")
                page.wait_for_function("document.getElementById('status-display').innerText.toLowerCase() === 'pass'", timeout=10000)

                # Test 6: Report JSON Export
                print("Testing report export download...")
                with page.expect_download() as download_info:
                    page.click("#btn-export")
                download = download_info.value
                download_path = download.path()
                
                with open(download_path, 'r', encoding='utf-8') as f:
                    downloaded_json = json.load(f)
                
                assert downloaded_json["status"] == "pass", "Downloaded JSON report status mismatch"
                assert downloaded_json["target_printer_profile"] == "Prusa_MK4", "Downloaded JSON report profile mismatch"
                print("Exported JSON report verified successfully.")

                # Test 7: Horizontal Overflow Layout Checks
                viewports = [
                    ("desktop", 1280, 800),
                    ("tablet", 768, 1024),
                    ("mobile", 375, 667)
                ]
                routes = [
                    "/",
                    "/docs/user_manual",
                    "/docs/api_reference",
                    "/docs/architecture"
                ]

                for device, w, h in viewports:
                    page.set_viewport_size({"width": w, "height": h})
                    print(f"Viewport set to {device} ({w}x{h})")
                    for route in routes:
                        page.goto(f"{base_url}{route}")
                        page.wait_for_load_state("networkidle")
                        
                        # Programmatically assert horizontal overflow scroll width is within client window width bounds
                        has_overflow = page.evaluate("document.documentElement.scrollWidth > window.innerWidth")
                        
                        if has_overflow:
                            scroll_w = page.evaluate("document.documentElement.scrollWidth")
                            client_w = page.evaluate("window.innerWidth")
                            print(f"FAIL: Responsive overflow detected on '{route}' at {device}! scrollWidth={scroll_w} > window.innerWidth={client_w}", file=sys.stderr)
                            tests_failed = True
                        else:
                            print(f"PASS: Layout verified on '{route}' at {device}.")

            except Exception as test_err:
                print(f"Acceptance test failure under {browser_name}: {test_err}", file=sys.stderr)
                tests_failed = True
            finally:
                browser.close()
                
    # 4. Cleanup and Shutdown
    print("Shutting down REST server...")
    server_proc.terminate()
    try:
        server_proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        print("Server failed to exit; killing force.")
        server_proc.kill()

    if not tested_any:
        print("Error: No browser could be launched for acceptance tests. Please run 'python -m playwright install' to setup chromium.", file=sys.stderr)
        sys.exit(1)

    if tests_failed:
        print("=== BROWSER ACCEPTANCE TESTS FAILED ===", file=sys.stderr)
        sys.exit(1)
        
    print("=== BROWSER ACCEPTANCE TESTS COMPLETED SUCCESSFULLY ===")
    sys.exit(0)

if __name__ == "__main__":
    main()
