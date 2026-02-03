set shell := ["bash", "-eux", "-o", "pipefail", "-c"]

default: list

list:
    just --list

fmt:
    treefmt

lint:

build:
