"use client";

import { useRspc } from "@/integration";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useContext, useState } from "react";

import { LoginContext } from "../../components/contexts/loginContext";

export default function SignIn() {
	const [username, setUsername] = useState("");
	const { setIsLoggedIn } = useContext(LoginContext);
	const [isLoading, setIsLoading] = useState(false);

	const rspc = useRspc();
	const { mutate } = rspc.useMutation("account.login");
	const router = useRouter();

	const handleSubmit = (event: React.FormEvent) => {
		event.preventDefault();
		setIsLoading(true);
		mutate(
			{ username: username },
			{
				onSuccess: (loginSuccess) => {
					console.log(loginSuccess);
					setIsLoggedIn(true);
					console.log("redirecting");
					router.push("/");
				},
				onError: (error) => {
					console.log("Error: ", error);
					router.push("/signUp");
				},
			}
		);
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
			<div className="bg-darkSidebar text-darkText rounded px-12 py-10 flex flex-col space-y-4 w-96">
				<h1 className="text-2xl font-bold">Login to String!</h1>
				<form className="flex flex-col space-y-6" onSubmit={handleSubmit}>
					<label>
						Username
						<br />
						<input
							required
							onChange={(e) => setUsername(e.target.value)}
							type="text"
							className="py-1 px-1 rounded bg-darkBackground w-full"
						/>
					</label>
					<button
						type="submit"
						className="py-2 hover:bg-darkHover rounded drop-shadow-lg bg-darkBackground text-white"
					>
						Login
					</button>
					<Link href="/signUp" className="text-darkText">
						Don't have an account? Sign up here!
					</Link>
				</form>
			</div>
		</div>
	);
}
