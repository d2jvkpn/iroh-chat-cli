#!/bin/make

SHELL := /bin/bash

secret_key:
	# head -c 32 /dev/urandom | xxd -p -c 32
	head -c 32 /dev/urandom | base64

test01:
	cargo run --bin test01

Alice:
	cargo run --bin iroh-chat-cli --  --name Alice open

Bob:
	cargo run --bin iroh-chat-cli -- --name Bob join \
	  $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

John:
	cargo run --bin iroh-chat-cli -- --name John join \
	  $$(awk 'NR==1{printf $$1}' configs/Bob.topic.ticket | base64 -w0)

share_file:
	cargo run --bin iroh-share-file -- share Cargo.toml

receive_file:
	cargo run --bin iroh-share-file -- receive \
	  $$(cat configs/share_file.bob.ticket) configs/Cargo.toml
