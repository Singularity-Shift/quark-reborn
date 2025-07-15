#! /bin/bash
 
# replace it with the network your contract lives on
NETWORK_APTOS=mainnet
# replace it with your contract address
CONTRACT_ADDRESS="0x0eabd22210b1b985d0c5ec1e4902608f00b92461486d97c5bba29479c70534a4"
# replace it with your module name, every .move file except move script has module_address::module_name {}
MODULE_ADMIN=admin
MODULE_USER=user
MODULE_GROUP=group

 
# save the ABI to a TypeScript file
echo "export const ADMIN_ABI = $(curl https://fullnode.$NETWORK_APTOS.aptoslabs.com/v1/accounts/$CONTRACT_ADDRESS/module/$MODULE_ADMIN| sed -n 's/.*"abi":\({.*}\).*}$/\1/p') as const" > QuarkAdminAbi.ts
echo "export const USER_ABI = $(curl https://fullnode.$NETWORK_APTOS.aptoslabs.com/v1/accounts/$CONTRACT_ADDRESS/module/$MODULE_USER| sed -n 's/.*"abi":\({.*}\).*}$/\1/p') as const" > QuarkUserAbi.ts
echo "export const GROUP_ABI = $(curl https://fullnode.$NETWORK_APTOS.aptoslabs.com/v1/accounts/$CONTRACT_ADDRESS/module/$MODULE_GROUP| sed -n 's/.*"abi":\({.*}\).*}$/\1/p') as const" > QuarkGroupAbi.ts
