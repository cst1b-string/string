// import { ChatButton } from "@/components/chatButton";
import { CircularChatButton } from "@/components/circularButton";
import { faker } from "@faker-js/faker";

function getChatNames(): string[] {
	return new Array(15).fill(true).map(() => faker.lorem.sentence({ min: 10, max: 30 }));
	// return ["David", "Group Chat!"];
}

export default function ChatSidebar() {
	const chatNames = getChatNames();
	return (
		// "no-scrollbar" defined in globals.css
			<div 
			// style = {{flex: "0 0 auto"}}
			// className="flex items-center justify-center flex-col overflow-auto bg-darkGrey h-full w-20 space-y-4 pt-2"> 
			className="grid place-items-center overflow-auto bg-darkGrey h-full w-20 space-y-4 pt-2"> 
				{chatNames.map((chatName, index) => (
					<CircularChatButton key={index} chatName={chatName} />
				))}
				<CircularChatButton chatName = "+"/>
			</div>
	);	
}
