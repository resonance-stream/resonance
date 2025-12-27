import { GraphQLClient } from 'graphql-request'

const API_URL = import.meta.env.VITE_API_URL || '/graphql'

export const graphqlClient = new GraphQLClient(API_URL, {
  credentials: 'include',
})

export function setAuthToken(token: string | null) {
  if (token) {
    graphqlClient.setHeader('Authorization', `Bearer ${token}`)
  } else {
    graphqlClient.setHeader('Authorization', '')
  }
}
