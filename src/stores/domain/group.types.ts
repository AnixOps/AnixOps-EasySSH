export interface GroupUIState {
  isLoading: boolean;
  error: string | null;
  selectedGroupId: string | null;
  expandedGroupIds: string[];
}

export const initialGroupUIState: GroupUIState = {
  isLoading: false,
  error: null,
  selectedGroupId: null,
  expandedGroupIds: [],
};
