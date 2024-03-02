"use client";

import { useRspc } from "@/integration";
import { redirect } from "next/navigation";
import { useState } from "react";

export default function SignUp() {
	const [password, setPassword] = useState("");
	const [confirmPassword, setConfirmPassword] = useState("");
	const [passwordsMatch, setPasswordsMatch] = useState(true);
	const hasAccount = false; // temporary, will be replaced with a backend check if the user has an account

	const rspc = useRspc();

	const handleSubmit = (event: React.FormEvent) => {
		event.preventDefault();
		setPasswordsMatch(password === confirmPassword);

		if (passwordsMatch) {
			// will become a mutation to create a new account
		}
	};

	if (hasAccount) {
		redirect("/");
	}

	return (
		<div className="py-6 flex justify-center">
			<div className="bg-white rounded px-12 py-10 flex flex-col space-y-4 w-96">
				<h1 className="text-2xl">Welcome to String!</h1>
				<p className="text-slate-500">
					A peer-to-peer social network focused on security and privacy. Enter a username and
					password to get started.
				</p>
				<form className="flex flex-col space-y-6" onSubmit={handleSubmit}>
					<label>
						Username
						<br />
						<input
							required
							type="text"
							className="py-1 px-1 rounded border border-slate-500 w-full"
						/>
					</label>
					<label>
						Password
						<br />
						<input
							required
							type="password"
							className="py-1 px-1 rounded border border-slate-500 w-full"
							value={password}
							onChange={(e) => setPassword(e.target.value)}
						/>
					</label>
					<label>
						Confirm Password
						<br />
						<input
							required
							type="password"
							className={`py-1 px-1 rounded border ${passwordsMatch ? "border-slate-500" : "border-red-500"} w-full`}
							value={confirmPassword}
							onChange={(e) => setConfirmPassword(e.target.value)}
						/>
						{!passwordsMatch && <p className="text-red-500">Passwords do not match</p>}
					</label>
					<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">
						Create Account
					</button>
				</form>
			</div>
		</div>
	);
}
