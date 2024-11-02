
# Maturin

Useful command line snippets:

```sh
maturin develop --uv && python examples/example_1.py
```

Note that I initially thought that I have to add `pip` to the `requirements.in`, because maturin errors when it doesn't find pip.
However, it looks like it actually can internally use `uv` as well:
- https://github.com/PyO3/maturin/issues/1959
- https://github.com/PyO3/maturin/pull/2015
- https://github.com/PyO3/maturin/pull/2015/files


# Deployment / Cross Compiling

## Python side

Recommended reading list:

- [PEP 513](https://peps.python.org/pep-0513/) which introduces the manylinux concepts.
- [PEP 571](https://peps.python.org/pep-0571/) which introduces the `manylinux2010` tag.
- [PEP 599](https://peps.python.org/pep-0599/) which introduces the `manylinux2014` tag.
- [PEP 600](https://peps.python.org/pep-0600/) which introduces the newer GLIC-version-specific tags.

PEP 513 mentions methods how to deal with situations that require depending on third party libraries.
Apart from static linking, the recommended solution is to bundle `.so`'s.
The `auditwheel` tool can be used to check wheels for manylinux compliance, but also to help with bundling `.so`'s.

Maturin builds on `auditwheel` and thus is generally able to produce manylinux conforming wheels including bundled third party `.so`'s.


## What are all these errors when (cross) compiling with external dependencies involved?

In general, Rust libraries typically use the `package-config` tool to locate system site libraries to link against during building.
When it comes to building insider docker containers and/or cross compiling, many things can go wrong, which is why one is often facing errors from `package-config`.

As an example, the `alsa-sys` crate for instance runs this `build.rs` script during building:
https://github.com/diwic/alsa-sys/blob/master/build.rs

When trying to build from inside one of the manylinux/musllinux docker containers the first hurdle is that the third parties libraries are not installed.
Thus, the first error one may encounter is that `package-config` complains that the library is just not installed.

This can be solved reasonably well using the `before-script-linux` mechanism of maturin's GitHub Action.

Note: In some images the `package-config` tool itself may not be installed, which can also be installed manually.

When there is no cross compiling involved (i.e., when the target architecture matches the docker container architecture), `package-config` should be able to locate the library now, and building should succeed.
The maturin side can then take care of packaging up the linked `.so` into the target wheel, as described by PEP 513.

## Cross-compilation

Things get really funny for actual cross compiling, i.e., where the target architecture does not match the docker container architecture.
The challenge here is that it would generally not be sound to just link against certain `.so` that exist inside the docker, because they are the library in a "wrong" architecture.
`package-config` comes with a general protection against doing that, and will error with something saying "pkg-config has not been configured for cross compilation".
See e.g.:

- https://github.com/rust-lang/pkg-config-rs/issues/109
- https://github.com/rust-lang/pkg-config-rs?tab=readme-ov-file#external-configuration-via-target-scoped-environment-variables
- https://docs.rs/pkg-config/latest/pkg_config/#cross-compilation

So just setting `PKG_CONFIG_ALLOW_CROSS=1` would only disable the check, but probably would lead nowhere.
What is needed is to actually install the libraries for that architecture, and then configure `package-config` to find/use them.

### Installing libraries for different architectures

As suggested [here](https://github.com/rust-cross/rust-musl-cross/issues/69#issuecomment-1272433482) the principle pattern should be something like this:

```sh
# in the example for armhf
sudo dpkg --add-architecture armhf
sudo apt update
sudo apt install -y libasound2-dev:armhf
```

Side note about the architecture names:
The names used by Debian/Ubuntu (`armhf`) may not be the names used by the manylinux platform tag convention (`armv7` or is it `armv7l`?), but typically reasonably similar to guess.
To see the available architectures on Ubuntu side, one can use the package search, e.g.:
https://packages.ubuntu.com/jammy/libasound2-dev

Now the snippet above unfortunately doesn't work that easily.
When doing that one will get "404 not found" errors during the `apt update`.
Apparently the reason is that it enables the armhf architecture for all sources entries, but some of the existing ones may not contain any armhf binaries.
Therefore it is necessary to actually manually massage the `sources.list`:
* Prefixing the existing entries with an `[arch=amd86]` so that they are only used for the host/container architecture.
* Manually added (a subset) of sources entries with an `[arch=armhf]` tag.

Some related questions that helped me to make it work:
* https://askubuntu.com/questions/1244195/apt-cant-find-package-libasound2-dev-20-04-arm64
* https://askubuntu.com/questions/430705/how-to-use-apt-get-to-download-multi-arch-library
* https://stackoverflow.com/questions/38002543/apt-get-update-returned-a-non-zero-code-100
  This one is actually misleading -- that's the error I got, but no answer mentions the possible issues of having `sources.list` entries for a different architecture that don't contain binaries.
* https://www.reddit.com/r/linuxquestions/comments/zopt3j/apt_giving_me_404_errors_when_i_try_and_add_a/
* https://askubuntu.com/questions/1307659/arm64-multi-arch-docker-build-fails-on-apt-update

### Using the installed libraries (different architecture) with package-config

Still to be figured out...
https://github.com/rust-lang/pkg-config-rs/issues/172

Currently, I'm at the point that I successfully installed `libasound2-dev:armhf` and trying a desperate `PKG_CONFIG_ALLOW_CROSS=1`.
But this errors with:

```
Could not run `PKG_CONFIG_ALLOW_SYSTEM_LIBS=1 PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 pkg-config --libs --cflags alsa`
The pkg-config command could not be found.

Most likely, you need to install a pkg-config package for your OS.
Try `apt install pkg-config`, or `yum install pkg-config`,
or `pkg install pkg-config`, or `apk add pkgconfig` depending on your distribution.

If you've already installed it, ensure the pkg-config command is one of the
directories in the PATH environment variable.

If you did not expect this build to link to a pre-installed system library,
then check documentation of the alsa-sys crate for an option to
build the library from source, or disable features or dependencies
that require pkg-config.
```

This kind of makes sense, because `libasound2-dev:armhf` probably didn't install itself in the default location?


### What about cross-rs?

I don't really understand if it would be usable with `maturin`:
https://github.com/PyO3/maturin/discussions/2287


## ALSA related

The library that we need is `libasound2.so`.

The corresponding package name seems to be:
- `libasound2-dev` for Debian/Ubuntu
- `alsa-lib-devel` for Fedora/CentOS

Some more findings:

- https://github.com/rust-cross/rust-musl-cross/issues/69
  Basically describes exactly my problem
- https://github.com/diwic/alsa-sys/issues/10
  Is static linking an option?
- https://github.com/cross-rs/cross/wiki/FAQ#linking-external-libraries
  This example looks interesting because it directly shows how to cross compile against `libasound2-dev:armhf`.
  However this is for `cross-rs`, so I'm not sure if that helps for building with `maturin`?
- https://stackoverflow.com/questions/57037550/alsa-linking-when-cross-compiling-rust-program-for-arm
  This looks interesting.


## dbus related

Library name: `libdbus-1-dev`

- https://github.com/cross-rs/wiki_assets/tree/main/FAQ/dbus
- https://github.com/diwic/dbus-rs/issues/399
- https://github.com/diwic/dbus-rs/pull/408
  Note that dbus-rs has a "vendored" build mode, which was also suggested for ALSA here: https://github.com/diwic/alsa-sys/issues/10#issuecomment-2182185812


## Maturin GitHub Actions

For overview of the images used, see:
https://github.com/PyO3/maturin-action?tab=readme-ov-file#manylinux-docker-container

Some notes / resources:

- https://github.com/PyO3/maturin-action/issues/276
- https://github.com/PyO3/maturin-action/discussions/273#discussioncomment-9828658
- https://github.com/astral-sh/uv/blob/ca92b55605fe37c354f42e1126185cae6e8d0d66/.github/workflows/build-binaries.yml#L227-L240


## Snippets for testing deployed package on PyPI

```sh
TMP_VENV_DIR=/tmp/venv_dir
rm -rf "$TMP_VENV_DIR"
virtualenv "$TMP_VENV_DIR"
. $TMP_VENV_DIR/bin/activate
pip install picoapp numpy
python -c "import picoapp; print(picoapp.__file__)"
python example/example_1.py
```


# PyO3

General resources:
- https://pyo3.rs/v0.21.2/types

To get around the limitation of using a Python callback from a `Send + 'static` Rust closure:
- https://github.com/PyO3/pyo3/discussions/3788#discussioncomment-8325882
- https://docs.rs/pyo3/0.21.2/pyo3/marker/struct.Python.html#method.with_gil