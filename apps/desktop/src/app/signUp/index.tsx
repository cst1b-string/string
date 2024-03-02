import { redirect } from "next/navigation";

const hasAccount = false;

export default function SignUp() {

	if (hasAccount) {
		redirect("/");
	}
	
	return (
		<div className="py-6 flex justify-center">
		<div className="bg-white rounded px-12 py-10 flex flex-col space-y-4 w-96">
			<h1 className="text-2xl">Welcome to String!</h1>
			<p className="text-slate-500">A peer-to-peer social network focused on security and privacy. Simply enter a display name below to get started.</p>
			<form className="flex flex-col space-y-6">
				<label>
					Display Name
					<br />
					<input type="text" className="py-1 px-1 rounded border border-slate-500 w-full"/>
				</label>
				<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Create Account</button>
				<br />
			</form>
		</div>	
		</div>
	);
}
