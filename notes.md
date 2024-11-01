
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