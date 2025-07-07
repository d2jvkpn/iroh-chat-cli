#!/bin/make

SHELL := /bin/bash

secret_key:
	head -c 32 /dev/urandom | base32 | tr 'A-Z' 'a-z' | sed 's/=*$//'
	#head -c 32 /dev/urandom | base64
	#head -c 32 /dev/urandom | xxd -p -c 32

check:
	cargo check --tests

test_bins:
	cargo test --bins --

test_utils:
	cargo test --lib -- utils --show-output

test01:
	cargo test --test test01

test02:
	cargo test --test test02

test03:
	cargo test --test test03

test04:
	cargo test --test test04 -- --nocapture

Alice:
	cargo run -- --name Alice open -w configs/Alice.topic.ticket

# $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

Bob:
	cargo run -- --name Bob join configs/Alice.topic.ticket -w configs/Bob.topic.ticket

John:
	cargo run -- --name John join configs/Bob.topic.ticket -w configs/John.topic.ticket

#share_file:
#	cargo run -- share Cargo.toml configs/share_file.bob.ticket

#receive_file:
#	cargo run -- receive configs/share_file.bob.ticket configs/Cargo.toml

release:
	cargo build --release && ls -alh target/release
