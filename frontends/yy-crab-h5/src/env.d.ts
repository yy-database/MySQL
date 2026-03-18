/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}

declare module '@sxo/vue' {
  export const SxoButton: any
  export const SxoAvatar: any
  export const SxoInput: any
  export const SxoTag: any
  export const SxoBadge: any
  export const SxoDropdown: any
  export const SxoDropdownItem: any
  export const SxoPopover: any
  export const SxoTooltip: any
  export const SxoSwitch: any
  export const createSxo: any
}

declare module 'axios' {
  export interface AxiosInstance {
    get: (url: string) => Promise<any>
    post: (url: string, data?: any) => Promise<any>
    put: (url: string, data?: any) => Promise<any>
    delete: (url: string) => Promise<any>
    interceptors: {
      response: {
        use: (
          onFulfilled: (response: any) => any,
          onRejected: (error: any) => any
        ) => void
      }
    }
  }
  export interface AxiosResponse {
    data: any
  }
}

declare module 'virtual:generated-layouts' {
  export function setupLayouts(routes: any): any
}

declare module 'virtual:generated-pages' {
  const routes: any
  export default routes
}

declare module '*.ftl?raw' {
  const content: string
  export default content
}

declare function definePage(options: any): any

interface ImportMetaEnv {
  readonly BASE_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
