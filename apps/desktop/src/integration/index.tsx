import { createClient } from "@rspc/client";
import { createReactQueryHooks } from "@rspc/react";
import { TauriTransport } from "@rspc/tauri";
import { QueryClient } from "@tanstack/react-query";
import { createContext, useContext, useMemo } from "react";

import { Procedures } from "./bindings";

export class Integration {
	static readonly instance = new Integration();

	// root tauri transport
	public readonly client = createClient<Procedures>({
		transport: new TauriTransport(),
	});

	// tanstack client
	public readonly queryClient = new QueryClient();
	// rspc client
	public readonly rspc = createReactQueryHooks<Procedures>();

	private constructor() {
		// empty to prevent instantiation
	}
}

export const IntegrationContext = createContext(Integration.instance);

/**
 * Use the integration instance.
 * @returns
 */
export const useIntegration = () => useContext(IntegrationContext);

/**
 * Use the rspc instance.
 * @returns
 */
export const useRspc = () => useIntegration().rspc;

/**
 * Integration provider.
 */
export const IntegrationProvider: React.FC<React.PropsWithChildren> = ({ children }) => {
	const integration = useMemo(() => Integration.instance, []);
	return (
		<IntegrationContext.Provider value={integration}>
			<integration.rspc.Provider
				client={integration.client}
				//@ts-expect-error Something is wrong with RSPC's types
				queryClient={integration.queryClient}
			>
				<>{children}</>
			</integration.rspc.Provider>
		</IntegrationContext.Provider>
	);
};
