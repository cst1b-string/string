"use client";

import { useRspc } from "@/integration";
import Link from "next/link";


export default function ChatSidebar() {

	const rspc = useRspc();
	const { data } = rspc.useQuery(['channel.list']);
	
	return (
		<div>
			<Link href="./newChat">
				<button className="bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded">
					"New Chat"
				</button>
			</Link>
			{data && data.map((channel) => (
				<button className="bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded">
					{channel.title}
				</button>
			))}
		</div>
	);
}
