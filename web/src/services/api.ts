import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginResponse {
  success: boolean;
}

export interface ServerStats {
  transport_mode: string;
  in_bytes: number;
  out_bytes: number;
  version: string;
}

export interface Client {
  id: string;
  auth_link_id: string;
  sdn_client_ip: string;
  network: string;
  netmask: string;
  used: boolean | null;
  is_exit_node: boolean;
  exit_node_id: string | null;
}

export interface UserDataResponse {
  email: string;
  role: string;
}

export const api = createApi({
  reducerPath: "api",
  baseQuery: fetchBaseQuery({
    baseUrl: import.meta.env.VITE_API_URL,
    credentials: "include",
  }),
  tagTypes: ["Clients"],
  endpoints: (build) => ({
    login: build.mutation<LoginResponse, LoginRequest>({
      query: (credentials) => ({
        url: "login",
        method: "POST",
        body: credentials,
      }),
    }),

    getServerStats: build.query<ServerStats, void>({
      query: () => ({ url: `server` }),
    }),

    getClients: build.query<Client[], void>({
      query: () => ({ url: `clients` }),
      providesTags: (result) =>
        result
          ? [
              ...result.map(({ id }) => ({ type: "Clients", id }) as const),
              { type: "Clients", id: "LIST" },
            ]
          : [{ type: "Clients", id: "LIST" }],
    }),

    createClient: build.mutation<
      Client,
      { sdn_client_ip: string; netmask: string }
    >({
      query: (data) => ({
        url: `clients`,
        method: "POST",
        body: data,
      }),
      invalidatesTags: [{ type: "Clients", id: "LIST" }],
    }),

    deleteClient: build.mutation<Client, { id: string }>({
      query: (data) => ({
        url: `clients`,
        method: "DELETE",
        body: data,
      }),
      invalidatesTags: [{ type: "Clients", id: "LIST" }],
    }),

    setExitNode: build.mutation<
      Client,
      { id: string; is_exit_node: boolean }
    >({
      query: ({ id, is_exit_node }) => ({
        url: `clients/${id}/exit-node`,
        method: "PATCH",
        body: { is_exit_node },
      }),
      invalidatesTags: [{ type: "Clients", id: "LIST" }],
    }),

    setUseExitNode: build.mutation<
      Client,
      { id: string; exit_node_id: string | null }
    >({
      query: ({ id, exit_node_id }) => ({
        url: `clients/${id}/use-exit-node`,
        method: "PATCH",
        body: { exit_node_id },
      }),
      invalidatesTags: [{ type: "Clients", id: "LIST" }],
    }),

    getUserData: build.query<UserDataResponse, void>({
      query: () => ({ url: `user` }),
    }),
  }),
});

export const {
  useLoginMutation,
  useGetServerStatsQuery,
  useGetClientsQuery,
  useCreateClientMutation,
  useGetUserDataQuery,
  useDeleteClientMutation,
  useSetExitNodeMutation,
  useSetUseExitNodeMutation,
} = api;
