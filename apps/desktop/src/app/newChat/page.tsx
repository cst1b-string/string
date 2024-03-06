"use client";

import { LoginContext } from "@/components/contexts/loginContext";
import { useRspc } from "@/integration";
import { useRouter } from "next/navigation";
import { useContext, useState } from "react";

export default function NewChat() {
	const [channel_title, setChannelTitle] = useState("");
	const rspc = useRspc();
	const { mutate } = rspc.useMutation("channel.create");
	const router = useRouter();
	const { isLoggedIn } = useContext(LoginContext);
	console.log("isLoggedIn new chat page: ", isLoggedIn);

	const handleCreateChannel = () => {
		mutate({ title: channel_title, network_id: 1 });
		router.push("/");
	};

	return (
		<div className="py-6 px-4 flex justify-center space-y-2">
			<div className="bg-formBlue text-white rounded px-12 py-10 grid divide-y divide-gray-400 space-y-10 w-[600px] l:w-[700px] xl:w-[850px]">
				<div className="">
					<h1 className="text-2xl">Create New Chat</h1>
					<br />
					<form className="flex flex-col space-y-6" onSubmit={handleCreateChannel}>
						<label>
							Chat Name
							<br />
							<input
								id="new_channel_title"
								onChange={(e) => setChannelTitle(e.target.value)}
								required
								maxLength={30}
								type="text"
								className="py-1 px-1 rounded bg-darkInput w-full"
							/>
						</label>
						<button
							type="submit"
							className="py-2 rounded drop-shadow-lg hover:bg-darkHover bg-darkInput text-white"
						>
							Create
						</button>
					</form>
				</div>
				<div className="">
					<br />
					<h1 className="text-2xl">Join Existing Chat</h1>
					<br />
					<form className="flex flex-col space-y-1">
						<label>
							Chat Link
							<br />
							<input type="text" className="py-1 px-1 rounded bg-buttonBlue w-full" />
						</label>
						<br />
						<button
							type="submit"
							className="py-2 rounded drop-shadow-lg hover:bg-hoverBlue bg-buttonBlue text-white"
						>
							Join
						</button>
					</form>
				</div>
			</div>
		</div>
	);
}
