"use client";

import { faker } from "@faker-js/faker";
import { useVirtualizer } from "@tanstack/react-virtual";
import React from "react";

import "./chat.css";

const sentences = new Array(1000).fill(true).map(() => "The same sentence over and over again."); // faker.lorem.sentence({ min: 50, max: 100 }));

export default function Home() {
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
			className="List"
			style={{
				position: "relative",
				left: "7.5%",
				height: 800,
				width: "85%",
				overflowY: "auto",
				contain: "strict",
			}}
		>
			<div
				style={{
					height: virtualizer.getTotalSize(),
					width: "100%",
					position: "relative",
				}}
			>
				<div
					style={{
						position: "absolute",
						top: 0,
						left: 0,
						width: "100%",
						transform: `translateY(${items[0]?.start ?? 0}px)`,
					}}
				>
					{items.map((virtualRow) => (
						<div
							key={virtualRow.key}
							data-index={virtualRow.index}
							ref={virtualizer.measureElement}
							className={virtualRow.index % 2 ? "ListItemOdd" : "ListItemEven"}
						>
							<div style={{ padding: "10px 0" }}>
								<div className="user">User {virtualRow.index % 2 ? "1" : "2"}: </div>
								{sentences[virtualRow.index]}
							</div>
						</div>
					))}
				</div>
			</div>
		</div>
	); //Amaan
}
