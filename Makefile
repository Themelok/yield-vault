.PHONY: start-localnet
start-localnet:
	solana-test-validator --account-dir ./dumps/accounts \
	--clone EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
	--clone KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD \
	--clone 7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF \
	--clone D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59 \
	--clone B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D \
	--clone Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6 \
	--clone 9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo \
	--clone MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA \
	--clone 4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8 \
	--clone 2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB \
	--clone 7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat \
	--clone 3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG \
	--url  mainnet-beta \
	--reset

.PHONY: abuild
abuild:
	anchor build \
	&& cp target/idl/yield_vault.json cli/idls/yield_vault.json \
	&& cp target/idl/yield_vault.json keeper/idls/yield_vault.json