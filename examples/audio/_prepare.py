"""
pyrun example/audio/_prepare.py

=======
Sources
=======

* https://freesound.org
* https://github.com/plopgrizzly/plopsnd
* https://github.com/lavenderdotpet/CC0-Public-Domain-Sounds

"""

import urllib.request
from pathlib import Path

import librosa
import numpy as np

URLS = {
    "electric_piano": "https://github.com/plopgrizzly/plopsnd/raw/refs/heads/master/NS/Bank0/2_ElectricPiano1_MidC.wav",
    # https://freesound.org/people/Streetpoptunez/sounds/413073/
    "electric_bass_amp": "https://cdn.freesound.org/previews/413/413073_7123687-lq.mp3",
    # https://freesound.org/people/Yarmonics/sounds/441853/
    "metal_click": "https://cdn.freesound.org/previews/441/441853_9109395-lq.mp3",
}


def main() -> None:

    path = Path(__file__).parent

    for name, url in URLS.items():
        print(f"Preparing '{name}'...")
        response = urllib.request.urlopen(url)
        data = response.read()

        extension = Path(url).suffix

        orig_output_path = path / (name + extension)
        orig_output_path.write_bytes(data)

        sr = 44100
        y, _ = librosa.load(orig_output_path, sr=sr, mono=True)

        npz_output_path = orig_output_path.with_suffix(".npz")
        np.savez(npz_output_path, y)


if __name__ == "__main__":
    main()
