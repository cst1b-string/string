import { ChatButton } from '@/components/chatButton';
//import fs from 'fs'; //use client issue

const folderPath = './src/app/logs';

function getChatNames(folderPath: string): string[] {
	try {
		const files = fs.readdirSync(folderPath);
		const chatNames = files.map((file) => file.replace('.txt', ''));
		return chatNames;
	} catch (error) {
		console.error('Error reading folder:', error);
		return [];
	}
}

export default function ChatSidebar() {
	const chatNames = getChatNames(folderPath);
	return (
		<div>
			<ChatButton chatName='New Chat +' />
			{chatNames.map((chatName, index) => (
				<ChatButton key={index} chatName={chatName} />
			))}
		</div>
	);
}
