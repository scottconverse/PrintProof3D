import os
import sys

def check_drift():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    
    # Essential concepts that must be present in both MD and HTML versions
    requirements = {
        "API Reference": {
            "files": ("API_REFERENCE.md", "api_reference.html"),
            "keywords": [
                "PrinterProfile",
                "MaterialProfile",
                "ValidationReport",
                "PrinterConnectionConfig",
                "SimulatorScenario",
                "klipper",
                "octo_print",
                "marlin_serial",
                "prusa_link",
                "rep_rap_firmware",
                "bambu_mqtt",
                "elegoo_sdcp",
                "creality_os",
                "anycubic_lan",
                "flash_forge_tcp",
                "PluginEngine",
                "LoadedPlugin",
                "export_validation_plugin!",
                "list-printers",
                "list-materials",
                "validate-printer-profile",
                "validate-material-profile",
                "check-compatibility",
                "generate-printer-profile",
                "generate-material-profile",
                "validate-profile-directory",
                "/validate/model",
                "/validate/gcode",
                "/profiles/inspect",
                "/profiles/validate/printer",
                "/profiles/validate/material",
                "/validate/compatibility",
            ]
        },
        "User Manual": {
            "files": ("USER_MANUAL.md", "user_manual.html"),
            "keywords": [
                "Printer Profile",
                "Material Profile",
                "Connection Configuration",
                "Validation Report",
                "preflight",
                "list-printers",
                "list-materials",
                "inspect-profile",
                "validate-printer-profile",
                "validate-material-profile",
                "validate-profile-directory",
                "generate-printer-profile",
                "generate-material-profile",
                "check-compatibility",
                "Claude Desktop",
            ]
        }
    }
    
    any_failed = False
    
    for doc_name, spec in requirements.items():
        md_name, html_name = spec["files"]
        md_path = os.path.join(root_dir, md_name)
        html_path = os.path.join(root_dir, html_name)
        
        if not os.path.exists(md_path) or not os.path.exists(html_path):
            print(f"[FAIL] Missing documentation files: {md_name} or {html_name}")
            any_failed = True
            continue
            
        with open(md_path, "r", encoding="utf-8") as f:
            md_content = f.read().lower()
            
        with open(html_path, "r", encoding="utf-8") as f:
            html_content = f.read().lower()
            
        print(f"\nChecking drift for {doc_name} ({md_name} <-> {html_name})...")
        
        missing_in_md = []
        missing_in_html = []
        
        for keyword in spec["keywords"]:
            kw_lower = keyword.lower()
            if kw_lower not in md_content:
                missing_in_md.append(keyword)
            if kw_lower not in html_content:
                missing_in_html.append(keyword)
                
        if missing_in_md:
            print(f"  [DRIFT] Missing keywords in {md_name}: {missing_in_md}")
            any_failed = True
        if missing_in_html:
            print(f"  [DRIFT] Missing keywords in {html_name}: {missing_in_html}")
            any_failed = True
            
        if not missing_in_md and not missing_in_html:
            print(f"  [PASS] {md_name} and {html_name} are fully synchronized.")
            
    if any_failed:
        print("\n[FAIL] Documentation drift check failed. Essential topics/keywords are out of sync.")
        return False
        
    print("\n[PASS] Documentation drift check passed successfully.")
    return True

if __name__ == "__main__":
    if not check_drift():
        sys.exit(1)
    sys.exit(0)
