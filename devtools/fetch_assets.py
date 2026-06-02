import urllib.request
import os

def download_file(url, filepath):
    print(f"Downloading {url} to {filepath}...")
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    urllib.request.urlretrieve(url, filepath)
    print("Done.")

def main():
    assets_dir = os.path.normpath(os.path.join("crates", "rest", "assets"))
    
    files = {
        "three.min.js": "https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js",
        "STLLoader.js": "https://cdn.jsdelivr.net/npm/three@0.128.0/examples/js/loaders/STLLoader.js",
        "OrbitControls.js": "https://cdn.jsdelivr.net/npm/three@0.128.0/examples/js/controls/OrbitControls.js"
    }
    
    for filename, url in files.items():
        filepath = os.path.join(assets_dir, filename)
        download_file(url, filepath)

if __name__ == "__main__":
    main()
