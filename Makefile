#!/bin/make

SHELL := /bin/bash

secret_key:
	head -c 32 /dev/urandom | xxd -p -c 32
	head -c 32 /dev/urandom | base64

test01:
	cargo run --bin test01

Alice:
	cargo run --bin iroh-chat-cli --  --name Alice -w configs/Alice.topic.ticket open

# $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

Bob:
	cargo run --bin iroh-chat-cli -- --name Bob -w configs/Bob.topic.ticket \
	  join $$(cat configs/Alice.topic.ticket)

John:
	cargo run --bin iroh-chat-cli -- --name John -w configs/Bob.John.ticket \
	  join $$(cat configs/Bob.topic.ticket)

share_file:
	cargo run --bin iroh-share-file -- share Cargo.toml

receive_file:
	cargo run --bin iroh-share-file -- receive \
	  $$(cat configs/share_file.bob.ticket) configs/Cargo.toml

release:
	cargo build --release
	ls -alh target/release
