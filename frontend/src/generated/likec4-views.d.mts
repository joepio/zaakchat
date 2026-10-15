/* prettier-ignore-start */
/* eslint-disable */

/******************************************************************************
 * This file was generated
 * DO NOT EDIT MANUALLY!
 ******************************************************************************/

import type { PropsWithChildren } from 'react'
import type { JSX } from 'react/jsx-runtime'
import type { LikeC4Model } from 'likec4/model'
import type { LayoutedView } from 'likec4/model'
import type {
  LikeC4ViewProps as GenericLikeC4ViewProps,
  ReactLikeC4Props as GenericReactLikeC4Props
} from 'likec4/react'

import type { Aux, SpecAux } from 'likec4/model';

export type $Specs = SpecAux<
  // Element kinds
  | "actor"
  | "component"
  | "container"
  | "queue"
  | "storage"
  | "system",
  // Deployment kinds
  never,
  // Relationship kinds
  never,
  // Tags
  never,
  // Metadata keys
  never
>

export type $Aux = Aux<
  "layouted",
  // Elements
  | "inwoner"
  | "medewerker"
  | "postmark"
  | "zaakchat"
  | "zaaksysteem"
  | "zaakchat.asyncapiSpec"
  | "zaakchat.backend"
  | "zaakchat.frontend"
  | "zaakchat.sled"
  | "zaakchat.tantivy"
  | "zaakchat.backend.authHandler"
  | "zaakchat.backend.emailHandler"
  | "zaakchat.backend.emailService"
  | "zaakchat.backend.eventsHandler"
  | "zaakchat.backend.queryHandler"
  | "zaakchat.backend.resourcesHandler"
  | "zaakchat.backend.schemasHandler"
  | "zaakchat.backend.sseBroadcaster"
  | "zaakchat.frontend.apiDocs"
  | "zaakchat.frontend.dashboard"
  | "zaakchat.frontend.schemaForms"
  | "zaakchat.frontend.searchUI"
  | "zaakchat.frontend.sseClient"
  | "zaakchat.frontend.zaakDetail",
  // Deployments
  never,
  // Views
  | "backendComponents"
  | "eventFlow"
  | "frontendComponents"
  | "index"
  | "zaakchatContainers",
  // Project ID
  "default",
  $Specs
>

export type $ElementId = $Aux['ElementId']
export type $DeploymentId = $Aux['DeploymentId']
export type $ViewId = $Aux['ViewId']

export type $ElementKind = $Aux['ElementKind']
export type $RelationKind = $Aux['RelationKind']
export type $DeploymentKind = $Aux['DeploymentKind']
export type $Tag = $Aux['Tag']
export type $Tags = readonly $Aux['Tag'][]
export type $MetadataKey = $Aux['MetadataKey']


declare function isLikeC4ViewId(value: unknown): value is $ViewId;

declare const likec4model: LikeC4Model<$Aux>;
declare function useLikeC4Model(): LikeC4Model<$Aux>;
declare function useLikeC4View(viewId: $ViewId): LayoutedView<$Aux>;

declare function LikeC4ModelProvider(props: PropsWithChildren): JSX.Element;

type IconRendererProps = {
  node: {
    id: string
    title: string
    icon?: string | undefined
  }
}
declare function RenderIcon(props: IconRendererProps): JSX.Element;

type LikeC4ViewProps = GenericLikeC4ViewProps<$Aux>;
declare function LikeC4View({viewId, ...props}: LikeC4ViewProps): JSX.Element;

type ReactLikeC4Props = GenericReactLikeC4Props<$Aux>
declare function ReactLikeC4({viewId, ...props}: ReactLikeC4Props): JSX.Element;

export {
  type LikeC4ViewProps,
  type ReactLikeC4Props,
  isLikeC4ViewId,
  useLikeC4Model,
  useLikeC4View,
  likec4model,
  LikeC4ModelProvider,
  LikeC4View,
  RenderIcon,
  ReactLikeC4
}
/* prettier-ignore-end */
