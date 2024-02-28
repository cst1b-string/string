import * as React from "react";
import * as ReactDOM from "react-dom/client";
import { RouterProvider, createBrowserRouter } from "react-router-dom";

import { Home } from "./app/page";
import { Navbar } from "./components/navbar";
import "./globals.css";
import { client, queryClient, rspc } from "./integration";

const Layout: React.FC<React.PropsWithChildren> = ({ children }) => (
	<rspc.Provider queryClient={queryClient} client={client}>
		<>
			<Navbar />
			{children}
		</>
	</rspc.Provider>
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
		<RouterProvider router={router} />
	</React.StrictMode>
);
