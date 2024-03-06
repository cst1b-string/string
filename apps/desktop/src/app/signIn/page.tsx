"use client";

import { useRspc } from "@/integration";
import { useRouter } from "next/navigation";
import { useContext, useState } from "react";

import { LoginContext } from "../loginContext";

export default function SignIn() {
	const [isLoading, setIsLoading] = useState(false);
	const [username, setUsername] = useState("");
	const { setIsLoggedIn } = useContext(LoginContext);

	const rspc = useRspc();
	const { error, isError } = rspc.useQuery(["account.login", { username }]);
	const router = useRouter();

	const handleSubmit = (event: React.FormEvent) => {
		event.preventDefault();
		if (!isError) {
			setIsLoading(true);
			setIsLoggedIn(true);
			setIsLoading(false);
			console.log("redirecting");
			router.push("/");
		} else {
			console.log("Error: ", error);
			router.push("/signIn");
		}
	};

	if (isLoading) {
		return (
			<div className="h-screen w-screen flex justify-center items-center">
				<div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-white"></div>
			</div>
		);
	}

	return (
		<div className="py-6 flex justify-center">
			<div className="bg-[#113355] text-white rounded px-12 py-10 flex flex-col space-y-4 w-96">
				<h1 className="text-2xl font-bold">Login to String!</h1>
				<form className="flex flex-col space-y-6" onSubmit={handleSubmit}>
					<label>
						Username
						<br />
						<input
							required
							onChange={(e) => setUsername(e.target.value)}
							type="text"
							className="py-1 px-1 rounded bg-[#335577] w-full"
						/>
					</label>
					<button
						type="submit"
						className="py-2 hover:bg-[#224466] rounded drop-shadow-lg bg-[#335577] text-white"
					>
						Login
					</button>
				</form>
			</div>
		</div>
	);
}
