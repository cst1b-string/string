"use client";

import { useRspc } from "@/integration";
import { faker } from "@faker-js/faker";
import { useVirtualizer } from "@tanstack/react-virtual";
import React from "react";

export default function ChatLog({ selectedChannel }: { selectedChannel: number }) {
	const parentRef = React.useRef<HTMLDivElement>(null);
	const rspc = useRspc();

	const sentences = rspc.useQuery(["channel.messages", selectedChannel]).data;
	const count = sentences ? sentences.length : 1;
	const virtualizer = useVirtualizer({
		count,
		getScrollElement: () => parentRef.current,
		estimateSize: () => 45,
	});

	const items = virtualizer.getVirtualItems();

	React.useEffect(() => {
		virtualizer.scrollToIndex(count - 1);
	}, [count, virtualizer]);

	return (
		<div
			ref={parentRef}
			// 90p defined in tailwind.config.ts
			className="overflow-y-auto relative h-90p"
			style={{
				contain: "strict",
			}}
		>
			<div
				className="relative w-full"
				style={{
					height: virtualizer.getTotalSize(),
				}}
			>
				<div
					className="absolute top-0 left-0 w-full"
					style={{
						transform: `translateY(${items[0]?.start ?? 0}px)`,
					}}
				>
					{items.map((virtualRow) => (
						<div
							key={virtualRow.key}
							// style={{ backgroundColor: virtualRow.index % 2 ? "#2a2a54" : "#212141" }}
							data-index={virtualRow.index}
							ref={virtualizer.measureElement}
							className={`${virtualRow.index % 2 === 0 ? "bg-transparent" : "bg-formBlue"}`}
						>
							<div className="py-2.5 relative w-11/12 left-2.5 text-white">
								<div className="font-bold display-inline">
									User {virtualRow.index % 2 ? "1" : "2"}:{" "}
								</div>
								<div className="font-normal">
									{sentences ? sentences[virtualRow.index]?.content : ""}{" "}
								</div>
							</div>
						</div>
					))}
				</div>
			</div>
		</div>
	); //Amaan
}
