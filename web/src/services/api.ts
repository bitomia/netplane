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
}

export interface Client {
  sdn_client_ip: string;
  netmask: string;
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

    getClients: build.query<Client, void>({
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
} = api;
