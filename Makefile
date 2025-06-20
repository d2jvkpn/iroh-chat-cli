SHELL := /bin/bash

secret_key:
	head -c 32 /dev/urandom | xxd -p -c 32

Alice:
	cargo run --bin chat --  --name Alice open

#Bob:
#	cargo run --bin chat -- --name Bob join \
#	  $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base64 -w0)

Bob:
	cargo run --bin chat -- --name Bob join \
	  $$(awk 'NR==1{printf $$1}' configs/Alice.topic.ticket | base32 -w0)

John:
	cargo run --bin chat -- --name John join \
	  $$(awk 'NR==1{printf $$1}' configs/Bob.topic.ticket | base32 -w0)

