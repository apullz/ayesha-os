import { useCallback, useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { StatusBar } from 'expo-status-bar';
import {
  useFonts,
  Quicksand_400Regular,
  Quicksand_500Medium,
  Quicksand_600SemiBold,
  Quicksand_700Bold,
} from '@expo-google-fonts/quicksand';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { BottomNav } from './src/components/BottomNav';
import { ChatScreen } from './src/screens/ChatScreen';
import { StarsScreen } from './src/screens/StarsScreen';
import { SettingsScreen } from './src/screens/SettingsScreen';
import { colors } from './src/theme';
import type { TabId } from './src/types';

export default function App() {
  const [fontsLoaded] = useFonts({
    Quicksand_400Regular,
    Quicksand_500Medium,
    Quicksand_600SemiBold,
    Quicksand_700Bold,
  });
  const [tab, setTab] = useState<TabId>('chat');

  const handleTabChange = useCallback((id: TabId) => {
    setTab(id);
  }, []);

  if (!fontsLoaded) {
    return <View style={[styles.flex, styles.loading]} />;
  }

  return (
    <SafeAreaProvider>
      <View style={[styles.flex, styles.app]}>
        <StatusBar style="dark" />
        {tab === 'chat' && <ChatScreen />}
        {tab === 'stars' && <StarsScreen />}
        {tab === 'settings' && <SettingsScreen />}
        <BottomNav active={tab} onChange={handleTabChange} />
      </View>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  flex: {
    flex: 1,
  },
  app: {
    backgroundColor: colors.background,
  },
  loading: {
    backgroundColor: colors.background,
  },
});
