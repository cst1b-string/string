
export default function SignIn() {

	return (
		<div className="py-6 flex justify-center">
		<div className="bg-white rounded px-12 py-10 flex flex-col space-y-4 w-96">
			<h1 className="text-2xl">Sign In</h1>
			<form className="flex flex-col space-y-6">
				<label>
					Username
					<br />
					<input type="text" className="py-1 px-1 rounded border border-slate-500 w-full"/>
				</label>
				<label className="flow-root">
					Password
					<br />
					<input type="password" className="py-1 px-1 rounded border border-slate-500 w-full"/>
					<a href="/reset-password" className="text-blue-400 float-right text-sm">Forgot Password?</a>
				</label>
				<br />
				<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Sign In</button>
				<br />
				<div className="text-center">
					Need an account? <a href="/sign-up" className="text-blue-400">Sign Up</a>
				</div>
			</form>
		</div>	
		</div>
	);
}
