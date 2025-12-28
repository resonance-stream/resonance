import { GraphQLClient } from 'graphql-request'

// Lazy-initialized GraphQL client to ensure window.location is available
let _graphqlClient: GraphQLClient | null = null

const getApiUrl = (): string => {
  if (import.meta.env.VITE_API_URL) {
    return import.meta.env.VITE_API_URL
  }
  // In browser, construct full URL from current origin
  if (typeof window !== 'undefined') {
    return `${window.location.origin}/graphql`
  }
  return 'http://localhost:4440/graphql'
}

const getClient = (): GraphQLClient => {
  if (!_graphqlClient) {
    _graphqlClient = new GraphQLClient(getApiUrl(), {
      credentials: 'include',
    })
  }
  return _graphqlClient
}

// Export a proxy that lazily initializes the client
export const graphqlClient = new Proxy({} as GraphQLClient, {
  get(_target, prop: string | symbol) {
    const client = getClient()
    const value = client[prop as keyof GraphQLClient]
    if (typeof value === 'function') {
      return value.bind(client)
    }
    return value
  },
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
