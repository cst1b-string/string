

export default function NewChat() {
	return (
		<div className="py-6 grid grid-flow-row justify-center space-y-2">
			<div className="bg-white rounded px-12 py-10 grid divide-y divide-gray-400 space-y-10 w-96">
				<div className="">
					<h1 className="text-2xl">Create New Chat</h1>
					<form className="flex flex-col space-y-6">
						<label>
							Chat Name
							<br />
							<input type="text" className="py-1 px-1 rounded border border-slate-500 w-full"/>
						</label>
						<br />
						<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Create</button>
					</form>
				</div>
				<div className="">
					<h1 className="text-2xl">Join Existing Chat</h1>
					<form className="flex flex-col space-y-1">
						<button className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Scan QR Code</button>
						<br /> Or
						<label>
							Chat Link
							<br />
							<input type="text" className="py-1 px-1 rounded border border-slate-500 w-full"/>
						</label>
						<br />
						<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Join</button>
					</form>
				</div>
			</div>		
		</div>
	)
}
