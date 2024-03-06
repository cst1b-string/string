"use client";

import { useRspc } from "@/integration";
import { useRouter } from "next/navigation";
import { useState } from "react";

export default function SignUp() {
	const [password, setPassword] = useState("");
	const [username, setUsername] = useState("");
	const [confirmPassword, setConfirmPassword] = useState("");
	const [passwordsMatch, setPasswordsMatch] = useState(true);
	const [isLoading, setIsLoading] = useState(false);

	const rspc = useRspc();
	const createAccount = rspc.useMutation("account.create");
	const router = useRouter();

	const handleSubmit = (event: React.FormEvent) => {
		event.preventDefault();
		setIsLoading(true);
		setPasswordsMatch(password === confirmPassword);

		if (passwordsMatch) {
			createAccount.mutate(
				{ username: username, passphrase: password },
				{
					onSuccess: (loginSuccess) => {
						console.log(loginSuccess);
						setIsLoading(false);
						console.log("redirecting");
						router.push("/signIn");
					},
				}
			);
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
				<h1 className="text-2xl font-bold">Welcome to String!</h1>
				<p>
					A peer-to-peer social network focused on security and privacy. Simply enter a username to
					get started.
				</p>
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
					<label>
						Password
						<br />
						<input
							required
							type="password"
							value={password}
							onChange={(e) => setPassword(e.target.value)}
							className="py-1 px-1 rounded bg-[#335577] w-full"
						/>
					</label>
					<label>
						Confirm Password
						<br />
						<input
							required
							type="password"
							value={confirmPassword}
							onChange={(e) => setConfirmPassword(e.target.value)}
							className="py-1 px-1 rounded bg-[#335577] w-full"
						/>
					</label>
					{!passwordsMatch && <p className="text-red-500">Passwords do not match</p>}
					<button
						type="submit"
						disabled={!passwordsMatch}
						className="py-2 hover:bg-[#224466] rounded drop-shadow-lg bg-[#335577] text-white"
					>
						Create Account
					</button>
				</form>
			</div>
		</div>
	);
}
