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
                if browser_name == "chromium":
                    browser = browser_launcher.launch(
                        headless=True,
                        args=["--disable-gpu", "--use-gl=swiftshader"]
                    )
                else:
                    browser = browser_launcher.launch(headless=True)
                tested_any = True
            except Exception as e:
                print(f"Skipping browser {browser_name}: Not installed or failed to launch. Details: {e}")
                continue

            try:
                # Set up page context
                context = browser.new_context(viewport={"width": 1280, "height": 800})
                page = context.new_page()

                # Add diagnostic event listeners to capture browser/network errors
                def log_console(msg):
                    print(f"[{browser_name} Console] {msg.type}: {msg.text}")
                def log_pageerror(err):
                    print(f"[{browser_name} Page Error] Uncaught error: {err}", file=sys.stderr)
                def log_requestfailed(req):
                    print(f"[{browser_name} Network Request Failed] {req.method} {req.url} - Error: {req.failure if req.failure else 'Unknown'}", file=sys.stderr)
                def log_response(res):
                    url = res.url
                    # Log profiles responses or any error status response
                    if "/profiles/" in url or res.status >= 400:
                        try:
                            body = res.text()
                        except Exception:
                            body = "<unable to read body>"
                        print(f"[{browser_name} Network Response] {res.status} {url} - Body: {body[:1000]}")

                page.on("console", log_console)
                page.on("pageerror", log_pageerror)
                page.on("requestfailed", log_requestfailed)
                page.on("response", log_response)

                # Navigate to dashboard
                print(f"Navigating to {base_url}/ ...")
                page.goto(f"{base_url}/")
                assert "PrintProof3D — Print Validation Dashboard" in page.title(), "Title mismatch"

                # Check Predefined Profiles are populated
                print("Checking predefined profiles dropdown list...")
                try:
                    page.wait_for_function("document.querySelectorAll('#select-printer option').length > 1", timeout=15000)
                    page.wait_for_function("document.querySelectorAll('#select-material option').length > 1", timeout=15000)
                except Exception as e:
                    # Capture exact counts and options HTML to print diagnostics before raising
                    printers_count = page.evaluate("document.querySelectorAll('#select-printer option').length")
                    materials_count = page.evaluate("document.querySelectorAll('#select-material option').length")
                    print(f"[{browser_name} Timeout Diagnostics] Predefined profiles wait failed. Timeout: {e}", file=sys.stderr)
                    print(f"[{browser_name} Timeout Diagnostics] Options counts: printers={printers_count}, materials={materials_count}", file=sys.stderr)
                    print(f"[{browser_name} Timeout Diagnostics] Printer select HTML: {page.locator('#select-printer').inner_html()}", file=sys.stderr)
                    print(f"[{browser_name} Timeout Diagnostics] Material select HTML: {page.locator('#select-material').inner_html()}", file=sys.stderr)
                    print(f"[{browser_name} Timeout Diagnostics] API input URL value: {page.locator('#input-api-url').input_value()}", file=sys.stderr)
                    try:
                        p_fetch_text = page.evaluate(f"fetch('{base_url}/profiles/printers').then(r => r.status + ' - ' + r.statusText).catch(err => err.message)")
                        print(f"[{browser_name} Timeout Diagnostics] Direct fetch /profiles/printers response: {p_fetch_text}", file=sys.stderr)
                    except Exception as fe:
                        print(f"[{browser_name} Timeout Diagnostics] Direct fetch printer exception: {fe}", file=sys.stderr)
                    raise e
                
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
                
                # Assert 3D Canvas is not blank by evaluating WebGL pixels if WebGL is supported
                print("Evaluating WebGL pixel buffer to verify non-blank canvas...")
                webgl_supported = page.evaluate("""() => {
                    try {
                        const canvas = document.createElement('canvas');
                        return !!(canvas.getContext('webgl2') || canvas.getContext('webgl'));
                    } catch (e) {
                        return false;
                    }
                }""")
                if webgl_supported:
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
                else:
                    print(f"[{browser_name}] WebGL is not supported in this environment/browser. Skipping non-blank canvas assertion.")

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

                # Test 8: WebGL Fallback Mode UI behavior
                print("Testing WebGL fallback mode...")
                fallback_context = browser.new_context(viewport={"width": 1280, "height": 800})
                fallback_page = fallback_context.new_page()
                fallback_page.add_init_script("window.WebGLRenderingContext = undefined;")
                
                # Navigate to dashboard
                fallback_page.goto(f"{base_url}/")
                fallback_page.wait_for_load_state("domcontentloaded")
                
                # Assert fallback overlay text is displayed
                overlay_text = fallback_page.locator("#visualizer-overlay").inner_text()
                assert "WebGL is disabled or unsupported" in overlay_text, f"Fallback mode text not displayed: {overlay_text}"
                
                # Load a model file and verify fallback mode handles it and displays coordinates
                fallback_page.set_input_files("#input-file", tetra_path)
                fallback_page.wait_for_timeout(1000)
                overlay_text_after = fallback_page.locator("#visualizer-overlay").inner_text()
                assert "Model file loaded successfully" in overlay_text_after, f"Fallback mode load text not found: {overlay_text_after}"
                assert "Dimensions:" in overlay_text_after, f"Fallback dimensions not shown: {overlay_text_after}"
                fallback_page.close()
                fallback_context.close()

                # Test 9: visualizer spinner loading state & validation reset state
                print("Testing spinner pending state and validation reset state...")
                page.fill("#input-api-token", token)
                page.select_option("#select-printer", label="Prusa MK4")
                page.select_option("#select-material", label="Polylactic Acid")
                page.set_input_files("#input-file", tetra_path)
                
                # Intercept validation requests to assert states during in-flight status
                def check_pending_states(route):
                    loader_visible = page.locator("#visualizer-loader").is_visible()
                    assert loader_visible, "Validation pending spinner not visible"
                    
                    status_text = page.locator("#status-display").inner_text()
                    assert "validating" in status_text.lower(), f"Reset state not shown in status display: {status_text}"
                    
                    issues_text = page.locator("#issues-list").inner_text()
                    assert "performing analysis checks" in issues_text.lower(), f"Reset state not shown in issues list: {issues_text}"
                    
                    route.continue_()
                    
                page.route("**/validate/*", check_pending_states)
                page.click("#btn-validate")
                
                # Wait for validation response to finish
                page.wait_for_function("document.getElementById('status-display').innerText.toLowerCase() === 'pass'", timeout=10000)
                page.unroute("**/validate/*")

                # Test 10: Token autocomplete attribute presence
                print("Testing token input autocomplete attribute...")
                token_autocomplete = page.locator("#input-api-token").get_attribute("autocomplete")
                assert token_autocomplete in ["new-password", "current-password", "off"], f"Unexpected or missing autocomplete attribute: {token_autocomplete}"

                # Test 11a: Oversized STL client-side failure (Rectangular)
                print("Testing oversized STL client-side failure (Rectangular)...")
                page.select_option("#select-printer", value="custom")
                
                tiny_rect_profile = {
                    "manufacturer": "Custom",
                    "model": "TinyRect",
                    "build_volume": {
                        "type": "rectangular",
                        "x": 2.0,
                        "y": 2.0,
                        "z": 2.0
                    }
                }
                tiny_rect_path = os.path.join(os.path.dirname(__file__), "..", "temp_tiny_rect.json")
                with open(tiny_rect_path, "w") as f:
                    json.dump(tiny_rect_profile, f)
                
                page.set_input_files("#input-custom-printer", tiny_rect_path)
                page.set_input_files("#input-file", tetra_path)
                
                page.click("#btn-validate")
                
                # Wait for alert banner to show
                page.wait_for_selector("#alert-banner", state="visible", timeout=5000)
                alert_text = page.locator("#alert-banner").inner_text()
                assert "Local Validation Failure:" in alert_text, f"Expected local validation failure alert, got: {alert_text}"
                
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "fail", f"Expected client-side fail status, got: {status_text}"
                
                issues_text = page.locator("#issues-list").inner_text()
                assert "MODEL_EXCEEDS_BUILD_VOLUME" in issues_text, f"Expected MODEL_EXCEEDS_BUILD_VOLUME issue code, got: {issues_text}"
                
                os.remove(tiny_rect_path)
                page.click("#alert-banner button") # Dismiss alert

                # Test 11b: Oversized STL client-side failure (Cylindrical)
                print("Testing oversized STL client-side failure (Cylindrical)...")
                tiny_cyl_profile = {
                    "manufacturer": "Custom",
                    "model": "TinyCyl",
                    "build_volume": {
                        "type": "cylindrical",
                        "diameter": 2.0,
                        "z": 2.0
                    }
                }
                tiny_cyl_path = os.path.join(os.path.dirname(__file__), "..", "temp_tiny_cyl.json")
                with open(tiny_cyl_path, "w") as f:
                    json.dump(tiny_cyl_profile, f)
                    
                page.set_input_files("#input-custom-printer", tiny_cyl_path)
                page.click("#btn-validate")
                
                # Wait for alert banner to show
                page.wait_for_selector("#alert-banner", state="visible", timeout=5000)
                alert_text = page.locator("#alert-banner").inner_text()
                assert "Local Validation Failure:" in alert_text, f"Expected local validation failure alert for cylindrical, got: {alert_text}"
                
                status_text = page.locator("#status-display").inner_text().strip().lower()
                assert status_text == "fail", f"Expected cylindrical client-side fail status, got: {status_text}"
                
                issues_text = page.locator("#issues-list").inner_text()
                assert "MODEL_EXCEEDS_BUILD_VOLUME" in issues_text, f"Expected MODEL_EXCEEDS_BUILD_VOLUME for cylindrical, got: {issues_text}"
                
                os.remove(tiny_cyl_path)
                page.click("#alert-banner button") # Dismiss alert

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
                        page.wait_for_load_state("domcontentloaded")
                        
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
