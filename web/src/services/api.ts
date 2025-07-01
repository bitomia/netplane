import { createApi, fetchBaseQuery } from '@reduxjs/toolkit/query/react';
import type { User, Project, Question, Answer } from '@keenchat/types';

export const api = createApi({
  reducerPath: 'api',
  baseQuery: fetchBaseQuery({ baseUrl: import.meta.env.VITE_API_URL }),
  endpoints: (build) => ({
    getSelfUser: build.query<User, void>({
      query: () => `user`
    }),
    getProjects: build.query<Project, void>({
      query: () => ({ url: `projects` }),
      providesTags: (result) =>
        result
          ?
          [
            ...result.map(({ id }) => ({ type: 'Projects', id }) as const),
            { type: 'Projects', id: 'LIST' },
          ] : [{ type: 'Projects', id: 'LIST' }],
    }),
    createProject: build.mutation<Project, { name: string }>({
      query: (data) => ({
        url: `projects`,
        method: "POST",
        body: data,
      }),
      invalidatesTags: [{ type: 'Projects', id: 'LIST' }],
    }),
    getDocuments: build.query<Document, void>({
      query: (projectId) => ({ url: `projects/${projectId}/documents` }),
      providesTags: (result) =>
        result
          ?
          [
            ...result.map(({ id }) => ({ type: 'Documents', id }) as const),
            { type: 'Documents', id: 'LIST' },
          ] : [{ type: 'Documents', id: 'LIST' }],

    }),
    getDocument: build.query<Document, { projectId: string, documentId: string }>({
      query: ({ projectId, documentId }) => ({ url: `projects/${projectId}/documents/${documentId}` }),
    }),
    deleteDocument: build.mutation<Document, { projectId: string, documentId: string }>({
      query: ({ projectId, documentId }) => ({
        url: `/projects/${projectId}/documents/${documentId}`,
        method: 'DELETE',
      }),
      invalidatesTags: [{ type: 'Documents', id: 'LIST' }],
    }),
    uploadDocument: build.mutation<Document, { projectId: string, body: FormData }>({
      query: ({ projectId, body }) => (
        {
          url: `/projects/${projectId}/documents`,
          method: 'POST',
          body,
          formData: true,
        }),
      invalidatesTags: [{ type: 'Documents', id: 'LIST' }],
    }),
    createEmbeddings: build.mutation<Project, { projectId: string }>({
      query: (projectId) => ({
        url: `/projects/${projectId}/embeddings`,
        method: 'POST',
      }),
      invalidatesTags: [{ type: 'Documents', id: 'LIST' }],
    }),
    queryDocuments: build.mutation<Answer, Question>({
      query: (question) => ({
        url: `/projects/${question.project_id}/query`,
        method: 'POST',
        body: { message: question.message },
      })
    })
  })
});

export const { useGetSelfUserQuery, useGetProjectsQuery, useCreateProjectMutation, useGetDocumentsQuery, useUploadDocumentMutation, useCreateEmbeddingsMutation, useQueryDocumentsMutation, useGetDocumentQuery, useDeleteDocumentMutation } = api;
