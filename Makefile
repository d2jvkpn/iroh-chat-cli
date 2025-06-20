SHELL := /bin/bash

secret_key:
	head -c 32 /dev/urandom | xxd -p -c 32

Alice:
	cargo run --bin iroch-chat-cli --  --name Alice open

Bob:
	cargo run --bin iroch-chat-cli -- --name Bob join \
	  $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

John:
	cargo run --bin iroch-chat-cli -- --name John join \
	  $$(awk 'NR==1{printf $$1}' configs/Bob.topic.ticket | base64 -w0)

send_file:
	cargo run --bin iroch-share-file -- send Cargo.toml

receive_file:
	cargo run --bin iroch-share-file -- receive $$(cat configs/send_file.bob.ticket) configs/Cargo.toml
