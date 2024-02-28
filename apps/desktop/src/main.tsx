import * as React from "react";
import * as ReactDOM from "react-dom/client";
import { RouterProvider, createBrowserRouter } from "react-router-dom";

import { Home } from "./app/page";
import { Navbar } from "./components/navbar";
import "./globals.css";
import { IntegrationProvider } from "./integration";

const Layout: React.FC<React.PropsWithChildren> = ({ children }) => (
	<IntegrationProvider>
		<Navbar />
		{children}
	</IntegrationProvider>
);

const router = createBrowserRouter([
	{
		path: "/",
		element: (
			<Layout>
				<Home />,
			</Layout>
		),
	},
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<IntegrationProvider>
			<Navbar />
			<RouterProvider router={router} />
		</IntegrationProvider>
	</React.StrictMode>
);
