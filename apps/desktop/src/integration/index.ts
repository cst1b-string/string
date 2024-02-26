import { createContext } from "react";

export class Integration {
	static readonly instance = new Integration();

	private constructor() {
		// empty to prevent instantiation
	}
}

export const IntegrationContext = createContext(Integration.instance);
