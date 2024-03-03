"use client";

import { useRspc } from "@/integration";
import { redirect } from "next/navigation";
import { useState } from "react";

export default function SignUp() {
	const [password, setPassword] = useState("");
	const [confirmPassword, setConfirmPassword] = useState("");
	const [passwordsMatch, setPasswordsMatch] = useState(true);
	const hasAccount = true; // temporary, will be replaced with a backend check if the user has an account

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
						<input required type="text" className="py-1 px-1 rounded bg-[#335577] w-full" />
					</label>
					<button
						type="submit"
						className="py-2 hover:bg-[#224466] rounded drop-shadow-lg bg-[#335577] text-white"
					>
						Create Account
					</button>
				</form>
			</div>
		</div>
	);
}
