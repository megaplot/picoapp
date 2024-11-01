
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

https://github.com/rust-lang/pkg-config-rs/issues/109


## ALSA specific findings

- https://github.com/rust-cross/rust-musl-cross/issues/69
  Basically describes exactly my problem
- https://github.com/diwic/alsa-sys/issues/10
  Is static linking an option?


## Maturin GitHub Actions

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