
sync-keys:
	anchor keys sync
	
build:
	anchor build 
	anchor run generate-clients

build-devnet:
	anchor build -- --features devnet
	anchor run generate-clients

deploy-devnet:
	anchor deploy --provider.cluster devnet

deploy-localnet:
	make build
	anchor deploy --provider.cluster localnet

anchor-test-skip:
	anchor test --skip-local-validator

set-config-localnet:
	solana config set --url localhost 

set-config-devnet:
	solana config set --url devnet

start-test-validator:
	solana-test-validator --reset

test:
	make build-devnet
	anchor test --skip-build
