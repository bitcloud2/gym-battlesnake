install:
	@maturin build --release --sdist
	@python -m pip install --upgrade pip
	@pip install . -v

clean:
	@find . -type f -name '*.pyc' -delete
	@find . -type d -name '__pycache__' | xargs rm -rf
	@rm -rf build/
	@rm -rf dist/
	@rm -f MANIFEST
	@rm -rf docs/build/
	@rm -f .coverage
	@rm -rf *.egg*
	@rm -rf .tox
	@rm -rf htmlcov

.PHONY: install clean