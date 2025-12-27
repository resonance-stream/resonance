import { GraphQLClient } from 'graphql-request'

const API_URL = import.meta.env.VITE_API_URL || '/graphql'

export const graphqlClient = new GraphQLClient(API_URL, {
  credentials: 'include',
})

export function setAuthToken(token: string | null): void {
  if (token) {
    graphqlClient.setHeader('Authorization', `Bearer ${token}`)
  } else {
    // Remove the Authorization header entirely when logging out
    // graphql-request's setHeaders replaces all headers, so we set an empty object
    // but preserve any other headers that might be needed
    graphqlClient.setHeaders({})
  }
}
