import { ExistingChat } from './components/existingChat';
import { NewChat } from './components/newChat';

export default function Home() {
	return (
		<div>
			< NewChat />
			< ExistingChat />
		</div>
	);
}
