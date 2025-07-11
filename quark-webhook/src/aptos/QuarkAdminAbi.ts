export const QuarkAdminAbi = {
  address: "0x2033b72957c2f0b66cf5be479a2aa098d5bf18c36477907eba8be39435f2811",
  name: "admin_v5",
  friends: [],
  exposed_functions: [
    {
      name: "accept_admin",
      visibility: "public",
      is_entry: true,
      is_view: false,
      generic_type_params: [],
      params: ["&signer"],
      return: [],
    },
    {
      name: "accept_reviewer_pending_admin",
      visibility: "public",
      is_entry: true,
      is_view: false,
      generic_type_params: [],
      params: ["&signer"],
      return: [],
    },
    {
      name: "get_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: [],
      return: ["address"],
    },
    {
      name: "get_pending_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: [],
      return: ["address"],
    },
    {
      name: "get_reviewer",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: [],
      return: ["address"],
    },
    {
      name: "get_reviewer_pending_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: [],
      return: ["address"],
    },
    {
      name: "is_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: ["address"],
      return: ["bool"],
    },
    {
      name: "is_pending_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: ["address"],
      return: ["bool"],
    },
    {
      name: "is_reviewer",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: ["address"],
      return: ["bool"],
    },
    {
      name: "is_reviewer_pending_admin",
      visibility: "public",
      is_entry: false,
      is_view: true,
      generic_type_params: [],
      params: ["address"],
      return: ["bool"],
    },
    {
      name: "set_pending_admin",
      visibility: "public",
      is_entry: true,
      is_view: false,
      generic_type_params: [],
      params: ["&signer", "address"],
      return: [],
    },
    {
      name: "set_reviewer_pending_admin",
      visibility: "public",
      is_entry: true,
      is_view: false,
      generic_type_params: [],
      params: ["&signer", "address"],
      return: [],
    },
  ],
  structs: [
    {
      name: "Admin",
      is_native: false,
      is_event: false,
      abilities: ["key"],
      generic_type_params: [],
      fields: [
        { name: "account", type: "address" },
        { name: "pending_admin", type: "0x1::option::Option<address>" },
        { name: "reviewer_account", type: "address" },
        {
          name: "reviewer_pending_admin",
          type: "0x1::option::Option<address>",
        },
      ],
    },
    {
      name: "Config",
      is_native: false,
      is_event: false,
      abilities: ["key"],
      generic_type_params: [],
      fields: [{ name: "coin_addr", type: "0x1::option::Option<address>" }],
    },
  ],
} as const;
