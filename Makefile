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

Alice:
	cargo run --bin iroh-chat-cli --  --name Alice -w configs/Alice.topic.ticket open

# $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

Bob:
	cargo run --bin iroh-chat-cli -- --name Bob -w configs/Bob.topic.ticket join \
	  $$(cat configs/Alice.topic.ticket)

John:
	cargo run --bin iroh-chat-cli -- --name John -w configs/John.topic.ticket join \
	  $$(cat configs/Bob.topic.ticket)

share_file:
	cargo run --bin iroh-share-file -- share Cargo.toml

receive_file:
	cargo run --bin iroh-share-file -- receive \
	  $$(cat configs/share_file.bob.ticket) configs/Cargo.toml

release:
	cargo build --release
	ls -alh target/release
