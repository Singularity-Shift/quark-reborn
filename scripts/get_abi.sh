#! /bin/bash
 
# replace it with the network your contract lives on
NETWORK_APTOS=testnet
# replace it with your contract address
CONTRACT_ADDRESS="0x02033b72957c2f0b66cf5be479a2aa098d5bf18c36477907eba8be39435f2811"
# replace it with your module name, every .move file except move script has module_address::module_name {}
MODULE_ADMIN=admin_v3
MODULE_USER=user_v3

 
# save the ABI to a TypeScript file
echo "export const ADMIN_ABI = $(curl https://fullnode.$NETWORK_APTOS.aptoslabs.com/v1/accounts/$CONTRACT_ADDRESS/module/$MODULE_ADMIN| sed -n 's/.*"abi":\({.*}\).*}$/\1/p') as const" > QuarkAdminAbi.ts
echo "export const USER_ABI = $(curl https://fullnode.$NETWORK_APTOS.aptoslabs.com/v1/accounts/$CONTRACT_ADDRESS/module/$MODULE_USER| sed -n 's/.*"abi":\({.*}\).*}$/\1/p') as const" > QuarkUserAbi.ts

