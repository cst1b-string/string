'use client';
import { useRspc } from "@/integration"
import { redirect } from "next/navigation";

function createChannel(channel_title: string, channel_network_id: number) {
	const rspc = useRspc();
	const { mutate } = rspc.useMutation('channel.create');
	mutate({ title: channel_title, network_id: channel_network_id })
	redirect('.')
}


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
							<input id="new_channel_title" required maxLength={30} type="text" className="py-1 px-1 rounded border border-slate-500 w-full"/>
						</label>
						<button type="submit" className="py-2 rounded drop-shadow-lg bg-blue-400 text-white">Create</button>
					</form>
				</div>
				<div className="pt-2">
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
