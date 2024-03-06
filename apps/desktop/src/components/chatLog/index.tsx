"use client";

import { useRspc } from "@/integration";
import React from "react";

export default function ChatLog({ selectedChannel }: { selectedChannel: number }) {
	const rspc = useRspc();

	const messages = rspc.useQuery(["channel.messages", selectedChannel]).data ?? [];

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
					{messages.map((row, index) => (
						<div
							key={index}
							style={{ backgroundColor: index % 2 ? "#2a2a54" : "#212141" }}
							className={`${index % 2 === 0 ? "bg-transparent" : "bg-formBlue"}`}
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
