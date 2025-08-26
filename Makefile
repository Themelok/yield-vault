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
	--url  mainnet-beta \
	--reset