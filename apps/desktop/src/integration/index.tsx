import { createClient } from "@rspc/client";
import { createReactQueryHooks } from "@rspc/react";
import { TauriTransport } from "@rspc/tauri";
import { QueryClient } from "@tanstack/react-query";

import { Procedures } from "./bindings";

// You must provide the generated types as a generic and create a transport (in this example we are using HTTP Fetch) so that the client knows how to communicate with your API.
export const client = createClient<Procedures>({
	// Refer to the integration your using for the correct transport.
	transport: new TauriTransport(),
});

export const queryClient = new QueryClient();
export const rspc = createReactQueryHooks<Procedures>();
