"use client";

import { useRspc } from "@/integration";
import React from "react";

export default function ChatLog({ selectedChannel }: { selectedChannel: number }) {
	const rspc = useRspc();

	const { data } = rspc.useQuery(["channel.messages", selectedChannel], { refetchInterval: 1000 });

	return (
		<div
			// 90p defined in tailwind.config.ts
			className="overflow-y-auto relative h-full"
			style={{
				contain: "strict",
				height: 400,
			}}
		>
			<div className="relative w-full">
				<div className="absolute top-0 left-0 w-full">
					{data?.map((row, index) => (
						<div
							key={index}
							className={`${index % 2 === 0 ? "bg-darkSelected" : "bg-darkCircularChatButton"}`}
						>
							<div className="py-2.5 relative w-11/12 left-2.5 text-white">
								<div className="font-bold display-inline">User {index % 2 ? "1" : "2"}: </div>
								<div className="font-normal">{row.content}</div>
							</div>
						</div>
					))}
				</div>
			</div>
		</div>
	); //Amaan
}
