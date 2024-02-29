"use client";

import { faker } from "@faker-js/faker";
import { useVirtualizer } from "@tanstack/react-virtual";
import React from "react";

import "./chat.css";

const sentences = new Array(100).fill(true).map(() => faker.lorem.sentence({ min: 10, max: 30 })); // "The same sentence over and over again."); //

export default function ChatLog() {
	const parentRef = React.useRef<HTMLDivElement>(null);

	const count = sentences.length;
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
			// className="List"
			// 90p defined in tailwind.config.ts
			className = "List relative h-90p overflow-y-auto"
			style={{
				// overflowY: "auto",
				contain: "strict",
			}}
		>
			<div
				className = "relative w-full"
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
							className={`${virtualRow.index % 2 === 0 ? "bg-darkGrey" : "bg-navbarGrey"}`}
						>
							<div className="py-2.5 relative w-11/12 left-2.5p">
								<div className="user">User {virtualRow.index % 2 ? "1" : "2"}: </div>
								<div className="chat">{sentences[virtualRow.index]} </div>
							</div>
						</div>
					))}
				</div>
			</div>
		</div>
	); //Amaan
}
