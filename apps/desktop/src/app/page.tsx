import { ExistingChat } from '@/components/existingChat';
import { NewChat } from '../components/newChat';
import fs from 'fs';

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

export default function Home() {
	const chatNames = getChatNames(folderPath);
	return (
		<div>
			<NewChat />
			{chatNames.map((chatName, index) => (
				<ExistingChat key={index} chatName={chatName} />
			))}
		</div>
	);
}
